use crate::{ordering, query_arguments_ext::QueryArgumentsExt};
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

static ORDER_TABLE_ALIAS: &str = "order_cmp";

type AliasedScalar = (ScalarFieldRef, String);
type MaybeAliasedScalar = (ScalarFieldRef, Option<String>);
#[derive(Debug)]
struct OrderDefinition {
    // Field on which to perform the ordering
    pub(crate) field_aliased: AliasedScalar,
    // Direction of the sort
    pub(crate) sort_order: SortOrder,
    // Final column identifier to be used for the scalar field to order by
    pub(crate) order_column: Column<'static>,
    // Foreign keys of the relations on which the order is performed
    pub(crate) fks: Option<Vec<MaybeAliasedScalar>>,
}

#[derive(Debug)]
struct OrderDefinitions {
    pub(crate) definitions: Vec<OrderDefinition>,
    pub(crate) joins: Vec<JoinData<'static>>,
}

/// Builds a cursor query condition based on the cursor arguments and if necessary a table that the condition depends on.
///
/// An example query for 4 order-by fields is:
/// ```sql
/// SELECT
///   `TestModel`.`id`
/// FROM
///   `TestModel`,
///   -- >>> Begin Part #1
///   (
///       SELECT
///           `TestModel`.`fieldA`,
///           `TestModel`.`fieldB`,
///           `TestModel`.`fieldC`,
///           `TestModel`.`fieldD`
///       FROM
///           `TestModel`
///       WHERE
///           (`TestModel`.`id`) = (4)
///   ) AS `order_cmp`
///   -- <<< End Part #1
/// WHERE
///   -- >>> Begin Part #2
///   (`TestModel`.`fieldA` = `order_cmp`.`fieldA` AND `TestModel`.`fieldB` = `order_cmp`.`fieldB` AND `TestModel`.`fieldC` = `order_cmp`.`fieldC` AND `TestModel`.`fieldD` <= `order_cmp`.`fieldD`)
///   OR
///   (`TestModel`.`fieldA` = `order_cmp`.`fieldA` AND `TestModel`.`fieldB` = `order_cmp`.`fieldB` AND `TestModel`.`fieldC` > `order_cmp`.`fieldC`)
///   OR
///   (`TestModel`.`fieldA` = `order_cmp`.`fieldA` AND `TestModel`.`fieldB` > `order_cmp`.`fieldB`)
///   OR
///   (`TestModel`.`fieldA` < `order_cmp`.`fieldA`)
///   -- <<< End Part #2
/// ORDER BY
///   `TestModel`.`fieldA` DESC,
///   `TestModel`.`fieldB` ASC,
///   `TestModel`.`fieldC` ASC,
///   `TestModel`.`fieldD` DESC;
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
pub fn build(query_arguments: &QueryArguments, model: &ModelRef) -> (Option<Table<'static>>, ConditionTree<'static>) {
    match query_arguments.cursor {
        None => (None, ConditionTree::NoCondition),
        Some(ref cursor) => {
            let cursor_fields: Vec<_> = cursor.fields().collect();
            let cursor_values: Vec<_> = cursor.pairs.iter().map(|(f, v)| f.value(v.clone())).collect();
            let cursor_columns: Vec<_> = cursor_fields.as_slice().as_columns().collect();
            let cursor_row = Row::from(cursor_columns);

            // Invariant: Cursors are unique. This means we can create a subquery to find at most one row
            // that contains all the values required for the odering row comparison (order_subquery).
            // That does _not_ mean that this retrieved row has an ordering unique across all records, because
            // that can only be true if the orderBy contains a combination of fields that are unique, or a single unique field.
            let cursor_condition = cursor_row.clone().equals(cursor_values.clone());

            // Orderings for this query. Influences which fields we need to fetch for comparing order fields.
            let OrderDefinitions { mut definitions, joins } = order_definitions(query_arguments, model);

            // Subquery to find the value of the order field(s) that we need for comparison. Builds part #1 of the query example in the docs.
            let order_subquery = definitions
                .iter()
                .fold(Select::from_table(model.as_table()), |select, definition| {
                    select.column(
                        definition
                            .order_column
                            .to_owned()
                            .alias(definition.field_aliased.1.to_owned()),
                    )
                })
                .so_that(cursor_condition);

            let order_subquery = joins.into_iter().fold(order_subquery, |acc, join| acc.left_join(join));

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
                            order_definition.order_column.to_owned(),
                        ));
                    }

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
                        let order_definition = tail.first().unwrap();

                        and_conditions.push(map_orderby_condition(order_definition, reverse, true));
                    } else {
                        let order_definition = tail.first().unwrap();
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
fn map_orderby_condition(order_definition: &OrderDefinition, reverse: bool, include_eq: bool) -> Expression<'static> {
    let (field, field_alias) = &order_definition.field_aliased;
    let cmp_column = Column::from((ORDER_TABLE_ALIAS, field_alias.to_owned()));
    let cloned_cmp_column = cmp_column.clone();
    // TODO: can we not clone..?
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
            .into()
    } else {
        order_expr
    };

    order_expr
}

fn map_equality_condition(field: &AliasedScalar, order_column: Column<'static>) -> Expression<'static> {
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

fn order_definitions(query_arguments: &QueryArguments, model: &ModelRef) -> OrderDefinitions {
    let mut joins = vec![];
    let mut orderings: Vec<OrderDefinition> = vec![];

    for (index, order_by) in query_arguments.order_by.iter().enumerate() {
        let (mut computed_joins, join_aliases, order_column) = ordering::compute_joins(order_by, index, model);

        joins.append(&mut computed_joins);

        let last_hops: Vec<_> = order_by.path.iter().rev().take(2).collect();
        let maybe_last_hop = last_hops.get(0);
        let maybe_before_last_hop = last_hops.get(1);

        let fks = maybe_last_hop.map(|rf| {
            rf.relation_info
                .fields
                .iter()
                .map(|fk_name| {
                    let fk_field = rf.model().fields().find_from_scalar(fk_name).unwrap();

                    if maybe_before_last_hop.is_some() {
                        let last_two_joins: Vec<_> = join_aliases.iter().rev().take(2).collect();
                        let before_last_join_alias = *last_two_joins.get(1).unwrap();

                        (fk_field, Some(before_last_join_alias.to_owned()))
                    } else {
                        (fk_field, None)
                    }
                })
                .collect()
        });

        let field_aliased = (
            order_by.field.clone(),
            format!("{}_{}_{}", order_by.field.model().name, order_by.field.name, index).to_owned(),
        );

        orderings.push(OrderDefinition {
            field_aliased,
            sort_order: order_by.sort_order,
            order_column,
            fks,
        });
    }

    if orderings.is_empty() {
        let definitions = model
            .primary_identifier()
            .scalar_fields()
            .map(|f| {
                let field_aliased = (f.clone(), f.db_name().to_owned());

                OrderDefinition {
                    field_aliased,
                    sort_order: SortOrder::Ascending,
                    order_column: f.as_column(),
                    fks: None,
                }
            })
            .collect();

        return OrderDefinitions { definitions, joins };
    }

    OrderDefinitions {
        definitions: orderings,
        joins,
    }
}
