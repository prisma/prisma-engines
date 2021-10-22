use crate::{join_utils::*, query_arguments_ext::QueryArgumentsExt};
use connector_interface::QueryArguments;
use itertools::Itertools;
use prisma_models::*;
use quaint::ast::*;

static ORDER_JOIN_PREFIX: &str = "orderby_";
static ORDER_AGGREGATOR_ALIAS: &str = "orderby_aggregator";

#[derive(Debug, Clone)]
pub struct OrderByDefinition {
    /// Final column identifier to be used for the scalar field to order by
    pub(crate) order_column: Expression<'static>,
    /// Defines ordering for an `ORDER BY` statement.    
    pub(crate) order_definition: OrderDefinition<'static>,
    /// Joins necessary to perform the order by
    pub(crate) joins: Vec<AliasedJoin>,
}

/// Builds all expressions for an `ORDER BY` clause based on the query arguments.
#[tracing::instrument(skip(query_arguments, base_model))]
pub fn build(
    query_arguments: &QueryArguments,
    base_model: &ModelRef, // The model the ordering will start from
) -> Vec<OrderByDefinition> {
    let needs_reversed_order = query_arguments.needs_reversed_order();

    // The index is used to differentiate potentially separate relations to the same model.
    query_arguments
        .order_by
        .iter()
        .enumerate()
        .map(|(index, order_by)| match order_by {
            OrderBy::Scalar(order_by) => build_order_scalar(order_by, base_model, needs_reversed_order, index),
            OrderBy::Aggregation(order_by) if order_by.is_scalar_aggregation() => {
                build_order_aggr_scalar(order_by, needs_reversed_order)
            }
            OrderBy::Aggregation(order_by) => build_order_aggr_rel(order_by, base_model, needs_reversed_order, index),
            OrderBy::Relevance(order_by) => build_order_relevance(order_by, needs_reversed_order),
        })
        .collect_vec()
}

fn build_order_scalar(
    order_by: &OrderByScalar,
    base_model: &ModelRef,
    needs_reversed_order: bool,
    index: usize,
) -> OrderByDefinition {
    let (joins, order_column) = compute_joins_scalar(order_by, index, base_model);
    let order: Option<Order> = Some(order_by.sort_order.into_order(needs_reversed_order));
    let order_definition: OrderDefinition = (order_column.clone().into(), order);

    OrderByDefinition {
        order_column: order_column.into(),
        order_definition,
        joins,
    }
}

fn build_order_relevance(order_by: &OrderByRelevance, needs_reversed_order: bool) -> OrderByDefinition {
    let columns: Vec<Expression> = order_by
        .fields
        .iter()
        .map(|sf| {
            if sf.is_required {
                sf.as_column().into()
            } else {
                // If the field is nullable, coalesce it with an empty string so that:
                // - if all fields are nullable, the relevance will return 0
                // - if only _some_ of the fields are nullable, it doesn't affect the relevance for fields that aren't null
                let coalesce_params: Vec<Expression> = vec![sf.as_column().into(), Value::text("").into()];

                coalesce(coalesce_params).into()
            }
        })
        .collect();
    let order_column: Expression = text_search_relevance(&columns, order_by.search.clone()).into();
    let order: Option<Order> = Some(order_by.sort_order.into_order(needs_reversed_order));
    let order_definition: OrderDefinition = (order_column.clone(), order);

    OrderByDefinition {
        order_column,
        order_definition,
        joins: vec![],
    }
}

fn build_order_aggr_scalar(order_by: &OrderByAggregation, needs_reversed_order: bool) -> OrderByDefinition {
    let order: Option<Order> = Some(order_by.sort_order.into_order(needs_reversed_order));
    let order_column = order_by.field.as_ref().unwrap().as_column();
    let order_definition: OrderDefinition = match order_by.sort_aggregation {
        SortAggregation::Count => (count(order_column.clone()).into(), order),
        SortAggregation::Avg => (avg(order_column.clone()).into(), order),
        SortAggregation::Sum => (sum(order_column.clone()).into(), order),
        SortAggregation::Min => (min(order_column.clone()).into(), order),
        SortAggregation::Max => (max(order_column.clone()).into(), order),
    };

    OrderByDefinition {
        order_column: order_column.into(),
        order_definition,
        joins: vec![],
    }
}

fn build_order_aggr_rel(
    order_by: &OrderByAggregation,
    base_model: &ModelRef,
    needs_reversed_order: bool,
    index: usize,
) -> OrderByDefinition {
    let order: Option<Order> = Some(order_by.sort_order.into_order(needs_reversed_order));
    let (joins, order_column) = compute_joins_aggregation(order_by, index, base_model);
    let order_definition: OrderDefinition = match order_by.sort_aggregation {
        SortAggregation::Count => {
            let exprs: Vec<Expression> = vec![order_column.clone().into(), Value::integer(0).into()];

            // We coalesce the order by expr to 0 so that if there's no relation,
            // `COALESCE(NULL, 0)` will return `0`, thus preserving the order
            (coalesce(exprs).into(), order)
        }
        _ => unreachable!("Order by relation aggregation other than count are not supported"),
    };

    OrderByDefinition {
        order_column: order_column.into(),
        order_definition,
        joins,
    }
}

pub fn compute_joins_aggregation(
    order_by: &OrderByAggregation,
    order_by_index: usize,
    base_model: &ModelRef,
) -> (Vec<AliasedJoin>, Column<'static>) {
    let join_prefix = format!("{}{}", ORDER_JOIN_PREFIX, order_by_index);
    let (last_hop, rest_hops) = order_by
        .path
        .split_last()
        .expect("An order by relation aggregation has to have at least one hop");
    let mut joins = rest_hops
        .iter()
        .map(|rf| compute_one2m_join(base_model, rf, join_prefix.as_str()))
        .collect_vec();

    let aggregation_type = match order_by.sort_aggregation {
        SortAggregation::Count => AggregationType::Count,
        _ => unreachable!("Order by relation aggregation other than count are not supported"),
    };
    // We perform the aggregation on the last join
    let last_aggr_join = compute_aggr_join(
        last_hop,
        aggregation_type,
        ORDER_AGGREGATOR_ALIAS,
        join_prefix.as_str(),
        joins.last(),
    );
    // This is the final column identifier to be used for the scalar field to order by.
    // `{last_join_alias}.{ORDER_AGGREGATOR_ALIAS}`
    let order_by_column = Column::from((last_aggr_join.alias.to_owned(), ORDER_AGGREGATOR_ALIAS.to_owned()));

    joins.push(last_aggr_join);

    (joins, order_by_column)
}

pub fn compute_joins_scalar(
    order_by: &OrderByScalar,
    order_by_index: usize,
    base_model: &ModelRef,
) -> (Vec<AliasedJoin>, Column<'static>) {
    let join_prefix = format!("{}{}", ORDER_JOIN_PREFIX, order_by_index);
    let joins = order_by
        .path
        .iter()
        .map(|rf| compute_one2m_join(base_model, rf, join_prefix.as_str()))
        .collect_vec();
    // This is the final column identifier to be used for the scalar field to order by.
    // - If we order by a scalar field on the base model, we simply use the model's scalar field. eg:
    //   `{modelTable}.{field}`
    // - If we order by some relations, we use the alias used for the last join, e.g.
    //   `{join_alias}.{field}`
    let order_by_column = if let Some(last_join) = joins.last() {
        Column::from((last_join.alias.to_owned(), order_by.field.db_name().to_owned()))
    } else {
        order_by.field.as_column()
    };

    (joins, order_by_column)
}
