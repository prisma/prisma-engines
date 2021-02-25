use crate::{ordering, query_arguments_ext::QueryArgumentsExt};
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

static ORDER_TABLE_ALIAS: &str = "order_cmp";

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
            let (mut order_definitions, joins) = order_definitions(query_arguments, model);

            // Subquery to find the value of the order field(s) that we need for comparison. Builds part #1 of the query example in the docs.
            let order_subquery = order_definitions
                .iter()
                .fold(Select::from_table(model.as_table()), |select, (_, column, _)| {
                    select.column(column.to_owned())
                })
                .so_that(cursor_condition);

            let order_subquery = joins.into_iter().fold(order_subquery, |acc, join| acc.left_join(join));

            let subquery_table = Table::from(order_subquery).alias(ORDER_TABLE_ALIAS);
            let len = order_definitions.len();
            let reverse = query_arguments.needs_reversed_order();

            // Builds part #2 of the example query.
            // If we only have one ordering, we only want a single, slightly different, condition of (orderField [<= / >=] cmp_field).
            let condition_tree = if len == 1 {
                let (field, column, order) = order_definitions.pop().unwrap();
                ConditionTree::Single(Box::new(map_orderby_condition(&field, column, &order, reverse, true)))
            } else {
                let or_conditions = (0..len).fold(Vec::with_capacity(len), |mut conditions_acc, n| {
                    let (head, tail) = order_definitions.split_at(len - n - 1);
                    let mut and_conditions = Vec::with_capacity(head.len() + 1);

                    for (field, order_column, _) in head {
                        and_conditions.push(map_equality_condition(field, order_column.to_owned()));
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
                        let (field, order_column, order) = tail.first().unwrap();

                        and_conditions.push(map_orderby_condition(
                            field,
                            order_column.to_owned(),
                            order,
                            reverse,
                            true,
                        ));
                    } else {
                        let (field, order_column, order) = tail.first().unwrap();
                        and_conditions.push(map_orderby_condition(
                            field,
                            order_column.to_owned(),
                            order,
                            reverse,
                            false,
                        ));
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
    field: &ScalarFieldRef,
    order_column: Column<'static>,
    order: &SortOrder,
    reverse: bool,
    include_eq: bool,
) -> Expression<'static> {
    let cmp_column = Column::from((ORDER_TABLE_ALIAS, field.db_name().to_owned()));

    let order_expr: Expression<'static> = match order {
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
    if !field.is_required {
        order_expr
            .or(field.as_column().is_null())
            .or(Column::from((ORDER_TABLE_ALIAS, field.db_name().to_owned())).is_null())
            .into()
    } else {
        order_expr
    }
}

fn map_equality_condition(field: &ScalarFieldRef, order_column: Column<'static>) -> Expression<'static> {
    let cmp_column = Column::from((ORDER_TABLE_ALIAS, field.db_name().to_owned()));

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
) -> (
    Vec<(ScalarFieldRef, Column<'static>, SortOrder)>,
    Vec<JoinData<'static>>,
) {
    let mut joins = vec![];
    let mut orderings = vec![];

    for (index, order_by) in query_arguments.order_by.iter().enumerate() {
        let (mut computed_joins, order_by_column) = ordering::compute_joins(order_by, model, index);

        joins.append(&mut computed_joins);
        
        orderings.push((order_by.field.clone(), order_by_column, order_by.sort_order))
    }

    if orderings.is_empty() {
        return (
            model
                .primary_identifier()
                .scalar_fields()
                .map(|f| (f.clone(), f.as_column(), SortOrder::Ascending))
                .collect(),
            joins,
        );
    }

    (orderings, joins)
}
