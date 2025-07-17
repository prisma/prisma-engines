use crate::{
    Context,
    join_utils::AliasedJoin,
    model_extensions::{AsColumn, AsColumns, AsTable, SelectionResultExt},
    ordering::OrderByDefinition,
};
use itertools::Itertools;
use quaint::ast::*;
use query_builder::QueryArgumentsExt;
use query_structure::*;

#[derive(Debug)]
struct CursorOrderDefinition {
    /// Direction of the sort
    pub(crate) sort_order: SortOrder,
    /// Column on which the top-level ORDER BY is performed
    pub(crate) order_column: Expression<'static>,
    /// Foreign keys of the relations on which the order is performed
    pub(crate) order_fks: Option<Vec<CursorOrderForeignKey>>,
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
/// SELECT `ModelA`.`id`
/// FROM `ModelA`
///   LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///     `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///   )
/// WHERE (
///     (
///       `ModelA`.`fieldA` = (
///         SELECT `ModelA`.`fieldA`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///       AND `ModelA`.`fieldB` = (
///         SELECT `ModelA`.`fieldB`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///       AND `ModelA`.`fieldC` = (
///         SELECT `ModelA`.`fieldC`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///       AND `orderby_3_ModelB`.`fieldD` <= (
///         SELECT `orderby_3_ModelB`.`fieldD`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///     )
///     OR (
///       `ModelA`.`fieldA` = (
///         SELECT `ModelA`.`fieldA`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///       AND `ModelA`.`fieldB` = (
///         SELECT `ModelA`.`fieldB`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///       AND `ModelA`.`fieldC` > (
///         SELECT `ModelA`.`fieldC`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///     )
///     OR (
///       `ModelA`.`fieldA` = (
///         SELECT `ModelA`.`fieldA`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///       AND `ModelA`.`fieldB` > (
///         SELECT `ModelA`.`fieldB`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///     )
///     OR (
///       `ModelA`.`fieldA` < (
///         SELECT `ModelA`.`fieldA`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///     )
///   )
/// ORDER BY `ModelA`.`fieldA` DESC,
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
///       `TestModel`.`fieldA` = (
///         SELECT `ModelA`.`fieldA`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///       OR (
///         SELECT `ModelA`.`fieldA`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       ) IS NULL
///       OR `TestModel`.`fieldA` IS NULL
///     )
///     AND -- ...
///   )
///   -- ...The other blocks (3, 2) in between, then the single condition block:
///   OR (
///     `TestModel`.`fieldA` < (
///         SELECT `ModelA`.`fieldA`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       )
///     OR (
///         SELECT `ModelA`.`fieldA`
///         FROM `ModelA`
///           LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///             `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///           )
///         WHERE (`ModelA`.`id`) = (?)
///       ) IS NULL
///     OR `TestModel`.`fieldA` IS NULL
///   )
///   -- ...
/// ```
/// When the ordering is performed on a nullable _relation_,
/// the conditions change in the same way as above, with the addition that foreign keys are also compared to NULL:
/// ```sql
///   -- ... The first (4 - condition) block:
///   AND (
///     `orderby_3_ModelB`.`id` <= (
///       SELECT `orderby_3_ModelB`.`fieldD`
///       FROM `ModelA`
///         LEFT JOIN `ModelB` AS `orderby_3_ModelB` ON (
///           `ModelA`.`modelB_id` = `orderby_3_ModelB`.`fieldD`
///         )
///       WHERE (`ModelA`.`id`) = (?)
///     )
///     OR `ModelA`.`modelB_id` IS NULL -- >>> Additional check for the nullable foreign key
///   )
/// ```
pub(crate) fn build(
    query_arguments: &QueryArguments,
    model: &Model,
    order_by_defs: &[OrderByDefinition],
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    match query_arguments.cursor {
        None => ConditionTree::NoCondition,
        Some(ref cursor) => {
            let cursor_fields: Vec<_> = cursor.as_scalar_fields().expect("Cursor fields contain non-scalars.");
            let cursor_values: Vec<_> = cursor.db_values(ctx);
            let cursor_columns: Vec<_> = cursor_fields.as_slice().as_columns(ctx).collect();
            let cursor_row = Row::from(cursor_columns);

            // Invariant: Cursors are unique. This means we can create a subquery to find at most one row
            // that contains all the values required for the ordering row comparison (order_subquery).
            // That does _not_ mean that this retrieved row has an ordering unique across all records, because
            // that can only be true if the orderBy contains a combination of fields that are unique, or a single unique field.
            let cursor_condition = cursor_row.clone().equals(cursor_values.clone());

            // Orderings for this query. Influences which fields we need to fetch for comparing order fields.
            let mut definitions = order_definitions(query_arguments, model, order_by_defs, ctx);

            // Subquery to find the value of the order field(s) that we need for comparison.
            let order_subquery = Select::from_table(model.as_table(ctx)).so_that(cursor_condition);

            let order_subquery = order_by_defs
                .iter()
                .flat_map(|j| &j.joins)
                .fold(order_subquery, |acc, join| acc.join(join.data.clone()));

            let len = definitions.len();
            let reverse = query_arguments.needs_reversed_order();

            // If we only have one ordering, we only want a single, slightly different, condition of (orderField [<= / >=] cmp_field).

            if len == 1 {
                let order_definition = definitions.pop().unwrap();
                ConditionTree::Single(Box::new(map_orderby_condition(
                    &order_subquery,
                    &order_definition,
                    reverse,
                    true,
                    ctx,
                )))
            } else {
                let or_conditions = (0..len).fold(Vec::with_capacity(len), |mut conditions_acc, n| {
                    let (head, tail) = definitions.split_at(len - n - 1);
                    let mut and_conditions = Vec::with_capacity(head.len() + 1);

                    for order_definition in head {
                        and_conditions.push(map_equality_condition(&order_subquery, order_definition));
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
                        and_conditions.push(map_orderby_condition(
                            &order_subquery,
                            order_definition,
                            reverse,
                            true,
                            ctx,
                        ));
                    } else {
                        and_conditions.push(map_orderby_condition(
                            &order_subquery,
                            order_definition,
                            reverse,
                            false,
                            ctx,
                        ));
                    }

                    conditions_acc.push(ConditionTree::And(and_conditions));
                    conditions_acc
                });

                ConditionTree::Or(or_conditions.into_iter().map(Into::into).collect())
            }
        }
    }
}

// A negative `take` value signifies that values should be taken before the cursor,
// requiring the correct comparison operator to be used to fit the reversed order.
fn map_orderby_condition(
    order_subquery: &Select<'static>,
    order_definition: &CursorOrderDefinition,
    reverse: bool,
    include_eq: bool,
    ctx: &Context<'_>,
) -> Expression<'static> {
    let cmp_column = order_subquery.clone().value(order_definition.order_column.clone());
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
            .or(Expression::from(cloned_cmp_column).is_null())
            .into()
    } else {
        order_expr
    };

    // Add OR statements for the foreign key fields too if they are nullable

    if let Some(fks) = &order_definition.order_fks {
        fks.iter()
            .filter(|fk| !fk.field.is_required())
            .fold(order_expr, |acc, fk| {
                let col = if let Some(alias) = &fk.alias {
                    Column::from((alias.to_owned(), fk.field.db_name().to_owned()))
                } else {
                    fk.field.as_column(ctx)
                }
                .is_null();

                acc.or(col).into()
            })
    } else {
        order_expr
    }
}

fn map_equality_condition(
    order_subquery: &Select<'static>,
    order_definition: &CursorOrderDefinition,
) -> Expression<'static> {
    let cmp_column = order_subquery.clone().value(order_definition.order_column.clone());
    let order_column = order_definition.order_column.to_owned();

    // If we have null values in the ordering or comparison row, those are automatically included because we can't make a
    // statement over their order relative to the cursor.
    if order_definition.on_nullable_fields {
        order_column
            .clone()
            .equals(cmp_column.clone())
            .or(Expression::from(cmp_column).is_null())
            .or(order_column.is_null())
            .into()
    } else {
        order_column.equals(cmp_column).into()
    }
}

fn order_definitions(
    query_arguments: &QueryArguments,
    model: &Model,
    order_by_defs: &[OrderByDefinition],
    ctx: &Context<'_>,
) -> Vec<CursorOrderDefinition> {
    if query_arguments.order_by.len() != order_by_defs.len() {
        unreachable!("There must be an equal amount of order by definition than there are order bys")
    }

    if query_arguments.order_by.is_empty() {
        return model
            .primary_identifier()
            .as_scalar_fields()
            .expect("Primary identifier has non-scalar fields.")
            .into_iter()
            .map(|f| CursorOrderDefinition {
                sort_order: SortOrder::Ascending,
                order_column: f.as_column(ctx).into(),
                order_fks: None,
                on_nullable_fields: !f.is_required(),
            })
            .collect();
    }

    query_arguments
        .order_by
        .iter()
        .enumerate()
        .zip(order_by_defs.iter())
        .map(|((_, order_by), order_by_def)| match order_by {
            OrderBy::Scalar(order_by) => cursor_order_def_scalar(order_by, order_by_def),
            OrderBy::ScalarAggregation(order_by) => cursor_order_def_aggregation_scalar(order_by, order_by_def),
            OrderBy::ToManyAggregation(order_by) => cursor_order_def_aggregation_rel(order_by, order_by_def),
            OrderBy::Relevance(order_by) => cursor_order_def_relevance(order_by, order_by_def),
        })
        .collect_vec()
}

/// Build a CursorOrderDefinition for an order by scalar
fn cursor_order_def_scalar(order_by: &OrderByScalar, order_by_def: &OrderByDefinition) -> CursorOrderDefinition {
    // If there are any ordering hops, this finds the foreign key fields for the _last_ hop (we look for the last one because the ordering is done the last one).
    // These fk fields are needed to check whether they are nullable
    // cf: part #2 of the SQL query above, when a field is nullable.
    let fks = foreign_keys_from_order_path(&order_by.path, &order_by_def.joins);

    CursorOrderDefinition {
        sort_order: order_by.sort_order,
        order_column: order_by_def.order_column.clone(),
        order_fks: fks,
        on_nullable_fields: !order_by.field.is_required(),
    }
}

/// Build a CursorOrderDefinition for an order by aggregation scalar
fn cursor_order_def_aggregation_scalar(
    order_by: &OrderByScalarAggregation,
    order_by_def: &OrderByDefinition,
) -> CursorOrderDefinition {
    let coalesce_exprs: Vec<Expression> = vec![order_by_def.order_column.clone(), Value::int32(0).into()];

    // We coalesce the order column to 0 when it's compared to the cmp table since the aggregations joins
    // might return NULL on relations that have no connected records
    let order_column: Expression = coalesce(coalesce_exprs).into();

    CursorOrderDefinition {
        sort_order: order_by.sort_order,
        order_column: order_column.clone(),
        order_fks: None,
        on_nullable_fields: false,
    }
}

/// Build a CursorOrderDefinition for an order by aggregation on relations
fn cursor_order_def_aggregation_rel(
    order_by: &OrderByToManyAggregation,
    order_by_def: &OrderByDefinition,
) -> CursorOrderDefinition {
    // If there are any ordering hop, this finds the foreign key fields for the _last_ hop (we look for the last one because the ordering is done the last one).
    // These fk fields are needed to check whether they are nullable
    // cf: part #2 of the SQL query above, when a field is nullable.
    let fks = foreign_keys_from_order_path(&order_by.path, &order_by_def.joins);

    let coalesce_exprs: Vec<Expression> = vec![order_by_def.order_column.clone(), Value::int32(0).into()];
    // We coalesce the order column to 0 when it's compared to the cmp table since the aggregations joins
    // might return NULL on relations that have no connected records
    let order_column: Expression = coalesce(coalesce_exprs).into();

    CursorOrderDefinition {
        sort_order: order_by.sort_order,
        order_column: order_column.clone(),
        order_fks: fks,
        on_nullable_fields: false,
    }
}

/// Build a CursorOrderDefinition for an order by relevance
fn cursor_order_def_relevance(order_by: &OrderByRelevance, order_by_def: &OrderByDefinition) -> CursorOrderDefinition {
    let order_column = &order_by_def.order_column;

    CursorOrderDefinition {
        sort_order: order_by.sort_order,
        order_column: order_column.clone(),
        order_fks: None,
        on_nullable_fields: false,
    }
}

fn foreign_keys_from_order_path(path: &[OrderByHop], joins: &[AliasedJoin]) -> Option<Vec<CursorOrderForeignKey>> {
    let (before_last_hop, last_hop) = take_last_two_elem(path);

    last_hop.map(|hop| {
        match hop {
            OrderByHop::Relation(rf) => {
                rf.scalar_fields()
                    .into_iter()
                    .map(|fk_field| {
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
                                let before_last_join = before_last_join
                                    .expect("There should be an equal amount of order by hops and joins");

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
            }
            OrderByHop::Composite(_) => unreachable!("SQL connectors don't have composite support."),
        }
    })
}

// Returns (before_last_elem, last_elem)
fn take_last_two_elem<T>(slice: &[T]) -> (Option<&T>, Option<&T>) {
    let len = slice.len();

    match len {
        0 => (None, None),
        1 => (None, slice.first()),
        _ => (slice.get(len - 2), slice.get(len - 1)),
    }
}
