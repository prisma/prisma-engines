use crate::{join_utils::*, query_arguments_ext::QueryArgumentsExt};
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

static ORDER_JOIN_PREFIX: &str = "orderby_";
static ORDER_AGGREGATOR_ALIAS: &str = "orderby_aggregator";

#[derive(Debug, Clone)]
pub struct OrderingJoins {
    // Joins necessary to perform the order by
    pub(crate) joins: Vec<AliasedJoin>,
    // Final column identifier to be used for the scalar field to order by
    pub(crate) order_column: Column<'static>,
}

/// Builds all expressions for an `ORDER BY` clause based on the query arguments.
#[tracing::instrument(skip(query_arguments, base_model))]
pub fn build(
    query_arguments: &QueryArguments,
    base_model: &ModelRef, // The model the ordering will start from
) -> (Vec<OrderDefinition<'static>>, Vec<OrderingJoins>) {
    let needs_reversed_order = query_arguments.needs_reversed_order();

    let mut order_definitions: Vec<OrderDefinition<'static>> = vec![];
    let mut ordering_joins = vec![];

    // The index is used to differentiate potentially separate relations to the same model.
    for (index, order_by) in query_arguments.order_by.iter().enumerate() {
        let (joins, order_column) = compute_joins(order_by, index, base_model);
        let order = Some(order_by.sort_order.into_order(needs_reversed_order));

        if joins.is_empty() && order_by.sort_aggregation.is_some() {
            match order_by.sort_aggregation.unwrap() {
                SortAggregation::Count => {
                    order_definitions.push((count(order_column.clone()).into(), order));
                }
                SortAggregation::Avg => {
                    order_definitions.push((avg(order_column.clone()).into(), order));
                }
                SortAggregation::Sum => {
                    order_definitions.push((sum(order_column.clone()).into(), order));
                }
                SortAggregation::Min => {
                    order_definitions.push((min(order_column.clone()).into(), order));
                }
                SortAggregation::Max => {
                    order_definitions.push((max(order_column.clone()).into(), order));
                }
            }
        } else {
            order_definitions.push((order_column.clone().into(), order));
        }

        ordering_joins.push(OrderingJoins { joins, order_column });
    }

    (order_definitions, ordering_joins)
}

pub fn compute_joins(
    order_by: &OrderBy,
    order_by_index: usize,
    base_model: &ModelRef,
) -> (Vec<AliasedJoin>, Column<'static>) {
    let join_prefix = format!("{}{}", ORDER_JOIN_PREFIX, order_by_index);
    let mut joins = vec![];
    let last_path = order_by.path.last();

    for rf in order_by.path.iter() {
        // If it's an order by aggregation, we change the last join to compute the aggregation
        if order_by.sort_aggregation.is_some() && Some(rf) == last_path {
            let sort_aggregation = order_by.sort_aggregation.unwrap();
            let aggregation_type = match sort_aggregation {
                SortAggregation::Count => AggregationType::Count { _all: true },
                _ => unreachable!("Order by relation aggregation other than count are not supported"),
            };
            let ordering_join = compute_aggr_join(
                rf,
                aggregation_type,
                ORDER_AGGREGATOR_ALIAS,
                join_prefix.as_str(),
                joins.last(),
            );

            joins.push(ordering_join);
        } else {
            let ordering_join = compute_one2m_join(base_model, rf, join_prefix.as_str());

            joins.push(ordering_join);
        }
    }

    // When doing an order by aggregation, we always alias the aggregator to <ORDER_AGGREGATOR_ALIAS>
    let order_by_column_alias = if order_by.sort_aggregation.is_some() {
        ORDER_AGGREGATOR_ALIAS.to_owned()
    } else {
        order_by.field.db_name().to_owned()
    };
    // This is the final column identifier to be used for the scalar field to order by.
    // - If it's on the base model with no hops, it's for example `modelTable.field`.
    // - If it is with several hops, it's the alias used for the last join, e.g.
    //   `{join_alias}.field`
    // - If it's with an order by aggregation, it's the alias used for the join + alias used for the aggregator. eg:
    //   `{join_alias}.{aggregator_alias}`
    let order_by_column = if let Some(join) = joins.last() {
        Column::from((join.alias.to_owned(), order_by_column_alias))
    } else {
        order_by.field.as_column()
    };

    (joins, order_by_column)
}
