use crate::{join_utils::AliasedJoin, ordering::OrderByDefinition, query_arguments_ext::QueryArgumentsExt};
use connector_interface::QueryArguments;
use itertools::Itertools;
use prisma_models::*;
use quaint::ast::*;

static ORDER_TABLE_ALIAS: &str = "order_cmp";

#[derive(Debug)]
struct CursorOrderDefinition {
    /// Direction of the sort
    pub(crate) sort_order: SortOrder,
    /// Column on which the top-level ORDER BY is performed
    pub(crate) order_column: Expression<'static>,
    /// Foreign keys of the relations on which the order is performed
    pub(crate) order_fks: Option<Vec<CursorOrderForeignKey>>,
    /// Column selected from the ORDER_TABLE_ALIAS cmp table and compared against the order_column
    pub(crate) cmp_column: Expression<'static>,
    /// Alias of the cmp_column
    pub(crate) cmp_column_alias: String,
    /// Indicates whether the ordering is performed on nullable field(s)
    pub(crate) on_nullable_fields: bool,
}

#[derive(Debug)]
struct CursorOrderForeignKey {
    field: ScalarFieldRef,
    alias: Option<String>,
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
///     OR `ModelA`.`modelB_id` IS NULL -- >>> Additional check for the nullable foreign key
///   )
/// ```
#[tracing::instrument(name = "build_cursor_condition", skip(query_arguments, model, order_by_defs))]
pub fn build(
    query_arguments: &QueryArguments,
    model: &ModelRef,
    order_by_defs: &[OrderByDefinition],
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
            let mut definitions = order_definitions(query_arguments, model, &order_by_defs);

            // Subquery to find the value of the order field(s) that we need for comparison. Builds part #1 of the query example in the docs.
            let order_subquery = definitions
                .iter()
                .fold(Select::from_table(model.as_table()), |select, definition| {
                    select.value(definition.cmp_column.clone())
                })
                .so_that(cursor_condition);

            let order_subquery = order_by_defs
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
                        and_conditions.push(map_equality_condition(&order_definition));
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
    let cmp_column = Column::from((ORDER_TABLE_ALIAS, order_definition.cmp_column_alias.to_owned()));
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
    let order_expr = if order_definition.on_nullable_fields {
        order_expr
            .or(cloned_order_column.is_null())
            .or(cloned_cmp_column.is_null())
            .into()
    } else {
        order_expr
    };

    // Add OR statements for the foreign key fields too if they are nullable
    let order_expr = if let Some(fks) = &order_definition.order_fks {
        fks.iter()
            .filter(|fk| !fk.field.is_required)
            .fold(order_expr, |acc, fk| {
                let col = if let Some(alias) = &fk.alias {
                    Column::from((alias.to_owned(), fk.field.db_name().to_owned()))
                } else {
                    fk.field.as_column()
                }
                .is_null();

                acc.or(col).into()
            })
    } else {
        order_expr
    };

    order_expr
}

fn map_equality_condition(order_definition: &CursorOrderDefinition) -> Expression<'static> {
    let cmp_column = Column::from((ORDER_TABLE_ALIAS, order_definition.cmp_column_alias.to_owned()));
    let order_column = order_definition.order_column.to_owned();

    // If we have null values in the ordering or comparison row, those are automatically included because we can't make a
    // statement over their order relative to the cursor.
    if order_definition.on_nullable_fields {
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
    order_by_defs: &[OrderByDefinition],
) -> Vec<CursorOrderDefinition> {
    if query_arguments.order_by.len() != order_by_defs.len() {
        unreachable!("There must be an equal amount of order by definition than there are order bys")
    }

    if query_arguments.order_by.is_empty() {
        return model
            .primary_identifier()
            .scalar_fields()
            .map(|f| CursorOrderDefinition {
                sort_order: SortOrder::Ascending,
                order_column: f.as_column().into(),
                order_fks: None,
                cmp_column: f.as_column().into(),
                cmp_column_alias: f.db_name().to_string(),
                on_nullable_fields: !f.is_required,
            })
            .collect();
    }

    query_arguments
        .order_by
        .iter()
        .enumerate()
        .zip(order_by_defs.iter())
        .map(|((index, order_by), order_by_def)| match order_by {
            OrderBy::Scalar(order_by) => cursor_order_def_scalar(order_by, order_by_def, index),
            OrderBy::Aggregation(order_by) if order_by.is_scalar_aggregation() => {
                cursor_order_def_aggregation_scalar(order_by, order_by_def, index)
            }
            OrderBy::Aggregation(order_by) => cursor_order_def_aggregation_rel(order_by, order_by_def, index),
            OrderBy::Relevance(order_by) => cursor_order_def_relevance(order_by, order_by_def, index),
        })
        .collect_vec()
}

/// Build a CursorOrderDefinition for an order by scalar
fn cursor_order_def_scalar(
    order_by: &OrderByScalar,
    order_by_def: &OrderByDefinition,
    index: usize,
) -> CursorOrderDefinition {
    // If there are any ordering hop, this finds the foreign key fields for the _last_ hop (we look for the last one because the ordering is done the last one).
    // These fk fields are needed to check whether they are nullable
    // cf: part #2 of the SQL query above, when a field is nullable.
    let fks = foreign_keys_from_order_path(&order_by.path, &order_by_def.joins);

    // Selected fields needs to be aliased in case there are two order bys on two different tables, pointing to a field of the same name.
    // eg: orderBy: [{ id: asc }, { b: { id: asc } }]
    // Without these aliases, selecting from the <ORDER_TABLE_ALIAS> tmp table would result in ambiguous field name
    let cmp_column_alias = format!("{}_{}_{}", &order_by.field.model().name, &order_by.field.name, index);

    CursorOrderDefinition {
        sort_order: order_by.sort_order,
        order_column: order_by_def.order_column.clone(),
        order_fks: fks,
        cmp_column: order_by_def.order_column.clone().alias(cmp_column_alias.clone()),
        cmp_column_alias,
        on_nullable_fields: !order_by.field.is_required,
    }
}

/// Build a CursorOrderDefinition for an order by aggregation scalar
fn cursor_order_def_aggregation_scalar(
    order_by: &OrderByAggregation,
    order_by_def: &OrderByDefinition,
    index: usize,
) -> CursorOrderDefinition {
    let field = order_by.field.as_ref().unwrap();
    // Selected fields needs to be aliased in case there are two order bys on two different tables, pointing to a field of the same name.
    // eg: orderBy: [{ id: asc }, { b: { id: asc } }]
    // Without these aliases, selecting from the <ORDER_TABLE_ALIAS> cmp table would result in ambiguous field name
    let cmp_column_alias = format!("aggr_{}_{}_{}", &field.model().name, &field.name, index);

    let coalesce_exprs: Vec<Expression> = vec![order_by_def.order_column.clone(), Value::integer(0).into()];
    // We coalesce the order column to 0 when it's compared to the cmp table since the aggregations joins
    // might return NULL on relations that have no connected records
    let order_column: Expression = coalesce(coalesce_exprs).into();

    CursorOrderDefinition {
        sort_order: order_by.sort_order,
        order_column: order_column.clone(),
        order_fks: None,
        cmp_column: order_column.alias(cmp_column_alias.clone()),
        cmp_column_alias,
        on_nullable_fields: false,
    }
}

/// Build a CursorOrderDefinition for an order by aggregation on relations
fn cursor_order_def_aggregation_rel(
    order_by: &OrderByAggregation,
    order_by_def: &OrderByDefinition,
    index: usize,
) -> CursorOrderDefinition {
    // If there are any ordering hop, this finds the foreign key fields for the _last_ hop (we look for the last one because the ordering is done the last one).
    // These fk fields are needed to check whether they are nullable
    // cf: part #2 of the SQL query above, when a field is nullable.
    let fks = foreign_keys_from_order_path(&order_by.path, &order_by_def.joins);

    // Selected fields needs to be aliased in case there are two order bys on two different tables, pointing to a field of the same name.
    // eg: orderBy: [{ id: asc }, { b: { id: asc } }]
    // Without these aliases, selecting from the <ORDER_TABLE_ALIAS> cmp table would result in ambiguous field name
    let cmp_column_alias = format!(
        "aggr_{}_{}",
        order_by.path.iter().map(|rf| rf.model().name.to_owned()).join("_"),
        index
    );

    let coalesce_exprs: Vec<Expression> = vec![order_by_def.order_column.clone(), Value::integer(0).into()];
    // We coalesce the order column to 0 when it's compared to the cmp table since the aggregations joins
    // might return NULL on relations that have no connected records
    let order_column: Expression = coalesce(coalesce_exprs).into();

    CursorOrderDefinition {
        sort_order: order_by.sort_order,
        order_column: order_column.clone(),
        order_fks: fks,
        cmp_column: order_column.alias(cmp_column_alias.clone()),
        cmp_column_alias,
        on_nullable_fields: false,
    }
}

/// Build a CursorOrderDefinition for an order by relevance
fn cursor_order_def_relevance(
    order_by: &OrderByRelevance,
    order_by_def: &OrderByDefinition,
    index: usize,
) -> CursorOrderDefinition {
    let order_column = &order_by_def.order_column;
    let cmp_column_alias = format!(
        "relevance_{}_{}",
        order_by.fields.iter().map(|sf| sf.name.as_str()).join("_"),
        index
    );

    CursorOrderDefinition {
        sort_order: order_by.sort_order,
        order_column: order_column.clone(),
        order_fks: None,
        cmp_column: order_column.clone().alias(cmp_column_alias.clone()),
        cmp_column_alias,
        on_nullable_fields: false,
    }
}

fn foreign_keys_from_order_path(
    path: &[RelationFieldRef],
    joins: &[AliasedJoin],
) -> Option<Vec<CursorOrderForeignKey>> {
    let (before_last_hop, last_hop) = take_last_two_elem(&path);

    last_hop.map(|rf| {
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
                        let (before_last_join, _) = take_last_two_elem(joins);
                        let before_last_join =
                            before_last_join.expect("There should be an equal amount of order by hops and joins");

                        CursorOrderForeignKey {
                            field: fk_field,
                            alias: Some(before_last_join.alias.to_owned()),
                        }
                    }
                    None => {
                        // If there is not more than one hop, then there's no need to alias the fk field
                        CursorOrderForeignKey {
                            field: fk_field,
                            alias: None,
                        }
                    }
                }
            })
            .collect_vec()
    })
}

// Returns (before_last_elem, last_elem)
fn take_last_two_elem<T>(slice: &[T]) -> (Option<&T>, Option<&T>) {
    let len = slice.len();

    match len {
        0 => (None, None),
        1 => (None, slice.get(0)),
        _ => (slice.get(len - 2), slice.get(len - 1)),
    }
}
