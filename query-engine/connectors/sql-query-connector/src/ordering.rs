use crate::{join_utils::*, model_extensions::*, query_arguments_ext::QueryArgumentsExt, Context};
use connector_interface::QueryArguments;
use itertools::Itertools;
use prisma_models::*;
use quaint::ast::*;

#[derive(Debug, Clone)]
pub(crate) struct OrderByDefinition<'a> {
    /// Final column identifier to be used for the scalar field to order by
    pub(crate) order_column: Expression<'static>,
    /// Defines ordering for an `ORDER BY` statement.
    pub(crate) order_definition: OrderDefinition<'static>,
    /// Joins necessary to perform the order by
    pub(crate) joins: Vec<&'a AliasedJoin>,
}

/// Builds all expressions for an `ORDER BY` clause based on the query arguments.
pub(crate) fn build<'a>(
    query_arguments: &'a QueryArguments,
    joins_ctx: &'a JoinsContext,
    ctx: &'a Context<'_>,
) -> Vec<OrderByDefinition<'a>> {
    let needs_reversed_order = query_arguments.needs_reversed_order();

    // The index is used to differentiate potentially separate relations to the same model.
    query_arguments
        .order_by
        .iter()
        .map(|order_by| match order_by {
            OrderBy::Scalar(order_by) => build_order_scalar(order_by, needs_reversed_order, joins_ctx, ctx),
            OrderBy::ScalarAggregation(order_by) => build_order_aggr_scalar(order_by, needs_reversed_order, ctx),
            OrderBy::ToManyAggregation(order_by) => build_order_aggr_rel(order_by, needs_reversed_order, joins_ctx),
            OrderBy::Relevance(order_by) => build_order_relevance(order_by, needs_reversed_order, ctx),
        })
        .collect_vec()
}

fn build_order_scalar<'a>(
    order_by: &'a OrderByScalar,
    needs_reversed_order: bool,
    joins_ctx: &'a JoinsContext,
    ctx: &'a Context<'_>,
) -> OrderByDefinition<'a> {
    let (joins, order_column) = compute_joins_scalar(order_by, joins_ctx, ctx);
    let order: Option<Order> = Some(into_order(
        &order_by.sort_order,
        order_by.nulls_order.as_ref(),
        needs_reversed_order,
    ));
    let order_definition: OrderDefinition = (order_column.clone().into(), order);

    OrderByDefinition {
        order_column: order_column.into(),
        order_definition,
        joins,
    }
}

fn build_order_relevance<'a>(
    order_by: &'a OrderByRelevance,
    needs_reversed_order: bool,
    ctx: &'a Context<'_>,
) -> OrderByDefinition<'a> {
    let columns: Vec<Expression> = order_by.fields.iter().map(|sf| sf.as_column(ctx).into()).collect();
    let order_column: Expression = text_search_relevance(&columns, order_by.search.clone()).into();
    let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
    let order_definition: OrderDefinition = (order_column.clone(), order);

    OrderByDefinition {
        order_column,
        order_definition,
        joins: vec![],
    }
}

fn build_order_aggr_scalar<'a>(
    order_by: &'a OrderByScalarAggregation,
    needs_reversed_order: bool,
    ctx: &'a Context<'_>,
) -> OrderByDefinition<'a> {
    let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
    let order_column = order_by.field.as_column(ctx);
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

fn build_order_aggr_rel<'a>(
    order_by: &'a OrderByToManyAggregation,
    needs_reversed_order: bool,
    joins_ctx: &'a JoinsContext,
) -> OrderByDefinition<'a> {
    let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
    let (joins, order_column) = compute_joins_aggregation(order_by, joins_ctx);
    let order_definition: OrderDefinition = match order_by.sort_aggregation {
        SortAggregation::Count => {
            let exprs: Vec<Expression> = vec![order_column.clone().into(), Value::integer(0).into()];

            // We coalesce the order by expr to 0 so that if there's no relation,
            // `COALESCE(NULL, 0)` will return `0`, thus preserving the order
            (coalesce(exprs).into(), order)
        }
        _ => unreachable!("Order by relation aggregation other than count are not supported"),
    };

    dbg!(&joins);

    OrderByDefinition {
        order_column: order_column.into(),
        order_definition,
        joins,
    }
}

fn compute_joins_aggregation<'a>(
    order_by: &OrderByToManyAggregation,
    joins_ctx: &'a JoinsContext,
) -> (Vec<&'a AliasedJoin>, Column<'static>) {
    let joins = joins_ctx.get_from_order_by_to_many(order_by).unwrap();
    let last_join = joins.last().unwrap();

    // This is the final column identifier to be used for the scalar field to order by.
    // `{last_join_alias}.{last_join_aggregator_alias}`
    let order_by_column = Column::from((
        last_join.alias.to_owned(),
        last_join.aggregator_alias.as_ref().unwrap().to_owned(),
    ));

    (joins, order_by_column)
}

pub(crate) fn compute_joins_scalar<'a>(
    order_by: &'a OrderByScalar,
    joins_ctx: &'a JoinsContext,
    ctx: &Context<'_>,
) -> (Vec<&'a AliasedJoin>, Column<'static>) {
    let joins = joins_ctx.get_from_order_by_scalar(order_by);

    // This is the final column identifier to be used for the scalar field to order by.
    // - If we order by a scalar field on the base model, we simply use the model's scalar field. eg:
    //   `{modelTable}.{field}`
    // - If we order by some relations, we use the alias used for the last join, e.g.
    //   `{join_alias}.{field}`
    let order_by_column = if let Some(last_join) = joins.as_ref().and_then(|j| j.last()) {
        Column::from((last_join.alias.to_owned(), order_by.field.db_name().to_owned()))
    } else {
        order_by.field.as_column(ctx)
    };

    (joins.unwrap_or_default(), order_by_column)
}

pub fn into_order(sort_order: &SortOrder, nulls_order: Option<&NullsOrder>, reverse: bool) -> Order {
    match (sort_order, nulls_order, reverse) {
        // Without NULLS order
        (SortOrder::Ascending, None, false) => Order::Asc,
        (SortOrder::Descending, None, false) => Order::Desc,

        // Without NULLS order reverse
        (SortOrder::Ascending, None, true) => Order::Desc,
        (SortOrder::Descending, None, true) => Order::Asc,

        // With NULLS order
        (SortOrder::Ascending, Some(NullsOrder::First), false) => Order::AscNullsFirst,
        (SortOrder::Ascending, Some(NullsOrder::Last), false) => Order::AscNullsLast,
        (SortOrder::Descending, Some(NullsOrder::First), false) => Order::DescNullsFirst,
        (SortOrder::Descending, Some(NullsOrder::Last), false) => Order::DescNullsLast,

        // With NULLS order reverse
        (SortOrder::Ascending, Some(NullsOrder::First), true) => Order::DescNullsLast,
        (SortOrder::Ascending, Some(NullsOrder::Last), true) => Order::DescNullsFirst,
        (SortOrder::Descending, Some(NullsOrder::First), true) => Order::AscNullsLast,
        (SortOrder::Descending, Some(NullsOrder::Last), true) => Order::AscNullsFirst,
    }
}
