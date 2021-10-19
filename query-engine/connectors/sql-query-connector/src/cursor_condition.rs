use crate::{ordering::OrderingJoins, query_arguments_ext::QueryArgumentsExt};
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

static ORDER_TABLE_ALIAS: &str = "order_cmp";

type AliasedScalar = (ScalarFieldRef, String);
type MaybeAliasedScalar = (ScalarFieldRef, Option<String>);

#[derive(Debug)]
struct CursorOrderDefinition {
    // Field on which to perform the ordering
    pub(crate) field_aliased: AliasedScalar,
    // Direction of the sort
    pub(crate) sort_order: SortOrder,
    // Final column identifier to be used for the scalar field to order by
    pub(crate) order_column: Expression<'static>,
    // Foreign keys of the relations on which the order is performed
    pub(crate) fks: Option<Vec<MaybeAliasedScalar>>,
}

/// Builds a cursor query condition based on the cursor arguments and if necessary a table that the condition depends on.
///
/// An example query for 4 order-by fields (where the last field is one2m relation) is:
///
/// ```sql
/// SELECT
///   `ModelA`.`id`
/// FROM
///   `ModelA`
///   LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///     `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///   ),
///   -- >>> Begin Part #1
///   (
///     SELECT
///       `ModelA`.`fieldA` AS `ModelA_fieldA_0`,
///       `ModelA`.`fieldB` AS `ModelA_fieldB_1`,
///       `ModelA`.`fieldC` AS `ModelA_fieldC_2`,
///       `orderby_3_ModelB`.`fieldD` AS `ModelB_fieldD_3`
///     FROM
///       `ModelA`
///       LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///         `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///       )
///     WHERE
///       (`ModelA`.`id`) = (?)
///   ) AS `order_cmp` -- <<< End Part #1
/// WHERE
///   -- >>> Begin Part #2
///   (
///     (
///       `ModelA`.`fieldA` = `order_cmp`.`ModelA_fieldA_0`
///       AND `ModelA`.`fieldB` = `order_cmp`.`ModelA_fieldB_1`
///       AND `ModelA`.`fieldC` = `order_cmp`.`ModelA_fieldC_2`
///       AND `orderby_3_ModelB`.`fieldD` <= `order_cmp`.`ModelB_fieldD_3`
///     )
///     OR (
///       `ModelA`.`fieldA` = `order_cmp`.`ModelA_fieldA_0`
///       AND `ModelA`.`fieldB` = `order_cmp`.`ModelA_fieldB_1`
///       AND `ModelA`.`fieldC` > `order_cmp`.`ModelA_fieldC_2`
///     )
///     OR (
///       `ModelA`.`fieldA` = `order_cmp`.`ModelA_fieldA_0`
///       AND `ModelA`.`fieldB` > `order_cmp`.`ModelA_fieldB_1`
///     )
///     OR (
///       `ModelA`.`fieldA` < `order_cmp`.`ModelA_fieldA_0`
///     )
///   )
///   -- <<< End Part #2
/// ORDER BY
///   `ModelA`.`fieldA` DESC,
///   `ModelA`.`fieldB` ASC,
///   `ModelA`.`fieldC` ASC,
///   `orderby_3_ModelB`.`fieldD` DESC
/// ```
///
/// The above assumes that all field are non-nullable. If a field is nullable, #2 conditions slighty change:
/// ```sql
///   -- ... The first (4 - condition) block:
///   (
///     (
///       `TestModel`.`fieldA` = `order_cmp`.`fieldA`
///       OR `order_cmp`.`fieldA` IS NULL
///       OR `TestModel`.`fieldA` IS NULL
///     )
///     AND -- ...
///   )
///   -- ...The other blocks (3, 2) in between, then the single condition block:
///   OR (
///     `TestModel`.`fieldA` < `order_cmp`.`fieldA`
///     OR `order_cmp`.`fieldA` IS NULL
///     OR `TestModel`.`fieldA` IS NULL
///   )
///   -- ...
/// ```
/// When the ordering is performed on a nullable _relation_,
/// the conditions change in the same way as above, with the addition that foreign keys are also compared to NULL:
/// ```sql
///   -- ... The first (4 - condition) block:
///   AND (
///     `orderby_3_ModelB`.`id` <= `order_cmp`.`ModelB_fieldD_3`
///     OR `main`.`ModelA`.`modelB_id` IS NULL -- >>> Additional check for the nullable foreign key
///   )
/// ```
#[tracing::instrument(name = "build_cursor_condition", skip(query_arguments, model, ordering_joins))]
pub fn build(
    query_arguments: &QueryArguments,
    model: &ModelRef,
    ordering_joins: &[OrderingJoins],
) -> (Option<Table<'static>>, ConditionTree<'static>) {
    match query_arguments.cursor {
        None => (None, ConditionTree::NoCondition),
        Some(ref cursor) => {
            let cursor_fields: Vec<_> = cursor.fields().collect();
            let cursor_values: Vec<_> = cursor.pairs.iter().map(|(f, v)| f.value(v.clone())).collect();
            let cursor_columns: Vec<_> = cursor_fields.as_slice().as_columns().collect();
            let cursor_row = Row::from(cursor_columns);

            // Invariant: Cursors are unique. This means we can create a subquery to find at most one row
            // that contains all the values required for the ordering row comparison (order_subquery).
            // That does _not_ mean that this retrieved row has an ordering unique across all records, because
            // that can only be true if the orderBy contains a combination of fields that are unique, or a single unique field.
            let cursor_condition = cursor_row.clone().equals(cursor_values.clone());

            // Orderings for this query. Influences which fields we need to fetch for comparing order fields.
            let mut definitions = order_definitions(query_arguments, model, &ordering_joins);

            // Subquery to find the value of the order field(s) that we need for comparison. Builds part #1 of the query example in the docs.
            let order_subquery = definitions
                .iter()
                .fold(Select::from_table(model.as_table()), |select, definition| {
                    select.value(
                        definition
                            .order_column
                            .clone()
                            .alias(definition.field_aliased.1.to_owned()),
                    )
                })
                .so_that(cursor_condition);

            let order_subquery = ordering_joins
                .iter()
                .flat_map(|j| &j.joins)
                .fold(order_subquery, |acc, join| acc.left_join(join.data.clone()));

            let subquery_table = Table::from(order_subquery).alias(ORDER_TABLE_ALIAS);
            let len = definitions.len();
            let reverse = query_arguments.needs_reversed_order();

            // Builds part #2 of the example query.
            // If we only have one ordering, we only want a single, slightly different, condition of (orderField [<= / >=] cmp_field).
            let condition_tree = if len == 1 {
                let order_definition = definitions.pop().unwrap();
                ConditionTree::Single(Box::new(map_orderby_condition(&order_definition, reverse, true)))
            } else {
                let or_conditions = (0..len).fold(Vec::with_capacity(len), |mut conditions_acc, n| {
                    let (head, tail) = definitions.split_at(len - n - 1);
                    let mut and_conditions = Vec::with_capacity(head.len() + 1);

                    for order_definition in head {
                        and_conditions.push(map_equality_condition(
                            &order_definition.field_aliased,
                            order_definition.order_column.clone(),
                        ));
                    }

                    let order_definition = tail.first().unwrap();

                    if head.len() == len - 1 {
                        // Special case where we build lte / gte, not lt / gt.
                        // - We use the combination of all order-by fields as comparator for the the cursor.
                        // - This isn't necessarily unique as a combination, i.e. doesn't guarantee stable sort order.
                        // - Only the first condition, which is done over the full length of the fields, can have the leniency
                        //   of equality, because if _all_ sorting fields up until the last one are identical _and_ the last field is identical,
                        //   then the comparison row has multiple identical records and we need to retrieve those for post-processing later (throwing
                        //   away records up until the cursor ID, but we can't do that in SQL, because we can't assume IDs to be linear).
                        //
                        // Example to illustrate the above:
                        // OrderBy: A ASC | B ASC | C DESC, cursor on 2.
                        // ID A B C
                        // 1  2 2 3
                        // 2  2 2 2 <- cursor
                        // 3  3 1 4
                        // 4  5 7 1
                        //
                        // The conditions we build to make sure that we only get (2, 2, 2), (3, 1, 4) and (5, 7, 1):
                        // `(A = 2 AND B = 2 AND C >= 2) OR (A = 2 AND B > 2) OR (A > 2)`
                        // If we would do `(A = 2 AND B >= 2)` as the middle statement, we suddenly get record with ID 1 a well. However, we can't do
                        // `(A = 2 AND B = 2 AND C > 2)` either, because then we'd miss out on the cursor row as well as possible duplicates coming after the cursor,
                        // which also need to be included in the result.
                        //
                        // Said differently, we handle all the cases in which the prefixes are equal to len - 1 to account for possible identical comparators,
                        // but everything else must come strictly "after" the cursor.
                        and_conditions.push(map_orderby_condition(order_definition, reverse, true));
                    } else {
                        and_conditions.push(map_orderby_condition(order_definition, reverse, false));
                    }

                    conditions_acc.push(ConditionTree::And(and_conditions));
                    conditions_acc
                });

                ConditionTree::Or(or_conditions.into_iter().map(Into::into).collect())
            };

            (Some(subquery_table), condition_tree)
        }
    }
}

// A negative `take` value signifies that values should be taken before the cursor,
// requiring the correct comparison operator to be used to fit the reversed order.
fn map_orderby_condition(
    order_definition: &CursorOrderDefinition,
    reverse: bool,
    include_eq: bool,
) -> Expression<'static> {
    let (field, field_alias) = &order_definition.field_aliased;
    let cmp_column = Column::from((ORDER_TABLE_ALIAS, field_alias.to_owned()));
    let cloned_cmp_column = cmp_column.clone();
    let order_column = order_definition.order_column.clone();
    let cloned_order_column = order_column.clone();

    let order_expr: Expression<'static> = match order_definition.sort_order {
        // If it's ASC but we want to take from the back, the ORDER BY will be DESC, meaning that comparisons done need to be lt(e).
        SortOrder::Ascending if reverse => {
            if include_eq {
                order_column.less_than_or_equals(cmp_column)
            } else {
                order_column.less_than(cmp_column)
            }
        }

        // If it's DESC but we want to take from the back, the ORDER BY will be ASC, meaning that comparisons done need to be gt(e).
        SortOrder::Descending if reverse => {
            if include_eq {
                order_column.greater_than_or_equals(cmp_column)
            } else {
                order_column.greater_than(cmp_column)
            }
        }

        SortOrder::Ascending => {
            if include_eq {
                order_column.greater_than_or_equals(cmp_column)
            } else {
                order_column.greater_than(cmp_column)
            }
        }

        SortOrder::Descending => {
            if include_eq {
                order_column.less_than_or_equals(cmp_column)
            } else {
                order_column.less_than(cmp_column)
            }
        }
    }
    .into();

    // If we have null values in the ordering or comparison row, those are automatically included because we can't make a
    // statement over their order relative to the cursor.
    let order_expr = if !field.is_required {
        order_expr
            .or(cloned_order_column.is_null())
            .or(cloned_cmp_column.is_null())
            .into()
    } else {
        order_expr
    };

    // Add OR statements for the foreign key fields too if they are nullable
    let order_expr = if let Some(fks) = &order_definition.fks {
        fks.iter()
            .filter(|(fk, _)| !fk.is_required)
            .fold(order_expr, |acc, (fk, alias)| {
                let col = if let Some(alias) = alias {
                    Column::from((alias.to_owned(), fk.db_name().to_owned()))
                } else {
                    fk.as_column()
                }
                .is_null();

                acc.or(col).into()
            })
    } else {
        order_expr
    };

    order_expr
}

fn map_equality_condition(
    field: &AliasedScalar,
    order_column: impl Comparable<'static> + Clone,
) -> Expression<'static> {
    let (field, field_alias) = field;
    let cmp_column = Column::from((ORDER_TABLE_ALIAS, field_alias.to_owned()));

    // If we have null values in the ordering or comparison row, those are automatically included because we can't make a
    // statement over their order relative to the cursor.
    if !field.is_required {
        order_column
            .clone()
            .equals(cmp_column.clone())
            .or(cmp_column.is_null())
            .or(order_column.is_null())
            .into()
    } else {
        order_column.equals(cmp_column).into()
    }
}

fn order_definitions(
    query_arguments: &QueryArguments,
    model: &ModelRef,
    ordering_joins: &[OrderingJoins],
) -> Vec<CursorOrderDefinition> {
    let mut orderings: Vec<CursorOrderDefinition> = vec![];

    for (index, order_by) in query_arguments.order_by.iter().enumerate() {
        let (last_hop, before_last_hop) = take_last_two_elem(&order_by.path);
        let joins_for_hop = ordering_joins.get(index).unwrap();

        // If there are any ordering hop, this finds the foreign key fields for the _last_ hop (we look for the last one because the ordering is done the last one).
        // These fk fields are needed to check whether they are nullable
        // cf: part #2 of the SQL query above, when a field is nullable.
        let fks = last_hop.map(|rf| {
            rf.relation_info
                .fields
                .iter()
                .map(|fk_name| {
                    let fk_field = rf.model().fields().find_from_scalar(fk_name).unwrap();

                    // If there are _more than one_ hop, we need to refer to the fk fields using the
                    // join alias of the hop _before_ the last hop. eg:
                    //
                    // ```sql
                    // SELECT ...
                    // FROM
                    //  "public"."ModelA"
                    //   LEFT JOIN "public"."ModelB" AS "orderby_0_ModelB" ON (
                    //     "public"."ModelA"."b_id" = "orderby_0_ModelB"."id"
                    //   )
                    //   LEFT JOIN "public"."ModelC" AS "orderby_0_ModelC" ON (
                    //     "orderby_0_ModelB"."c_id" = "orderby_0_ModelC"."id"
                    //   )
                    // WHERE
                    // ( ... OR <before_last_join_alias>.<foreign_key_db_name> IS NULL )
                    // ```
                    // In the example above, <before_last_join_alias> == "orderby_0_ModelB"
                    //                          <foreign_key_db_name> == "c_id"
                    match before_last_hop {
                        Some(_) => {
                            let (_, before_last_join) = take_last_two_elem(&joins_for_hop.joins);
                            let before_last_join =
                                before_last_join.expect("There should be an equal amount of order by hops and joins");

                            (fk_field, Some(before_last_join.alias.to_owned()))
                        }
                        None => {
                            // If there is not more than one hop, then there's no need to alias the fk field
                            (fk_field, None)
                        }
                    }
                })
                .collect()
        });

        // Selected fields needs to be aliased in case there are two order bys on two different tables, pointing to a field of the same name.
        // eg: orderBy: [{ id: asc }, { b: { id: asc } }]
        // Without these aliases, selecting from the <ORDER_TABLE_ALIAS> tmp table would result in ambiguous field name
        let field_aliased = (
            order_by.field.clone(),
            format!("{}_{}_{}", order_by.field.model().name, order_by.field.name, index).to_owned(),
        );

        // We coalesce the order_column if it's an order by aggregate
        // To prevent from having null comparisons against the `order_cmp` table
        let order_column = if order_by.sort_aggregation.is_some() {
            let coalesce_exprs: Vec<Expression> =
                vec![joins_for_hop.order_column.clone().into(), Value::integer(0).into()];

            coalesce(coalesce_exprs).into()
        } else {
            joins_for_hop.order_column.clone().into()
        };

        orderings.push(CursorOrderDefinition {
            field_aliased,
            sort_order: order_by.sort_order,
            order_column,
            fks,
        });
    }

    if orderings.is_empty() {
        return model
            .primary_identifier()
            .scalar_fields()
            .map(|f| {
                let field_aliased = (f.clone(), f.db_name().to_owned());

                CursorOrderDefinition {
                    field_aliased,
                    sort_order: SortOrder::Ascending,
                    order_column: f.as_column().into(),
                    fks: None,
                }
            })
            .collect();
    }

    orderings
}

// Returns (last_elem, before_last_elem)
fn take_last_two_elem<T>(slice: &[T]) -> (Option<&T>, Option<&T>) {
    let len = slice.len();

    match len {
        0 => (None, None),
        1 => (slice.get(0), None),
        _ => (slice.get(len - 1), slice.get(len - 2)),
    }
}
