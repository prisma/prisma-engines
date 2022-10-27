use crate::{
    join_utils::*, model_extensions::*, query_arguments_ext::QueryArgumentsExt, query_builder::QueryBuilderContext,
};
use connector_interface::{NestedRead, QueryArguments};
use itertools::Itertools;
use prisma_models::*;
use quaint::ast::*;

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
pub fn build(
    ctx: &mut QueryBuilderContext,
    query_arguments: &QueryArguments,
    nested_reads: &[NestedRead],
) -> Vec<OrderByDefinition> {
    let needs_reversed_order = query_arguments.needs_reversed_order();
    let nested_order_bys = process_nested_order_bys(nested_reads);

    query_arguments
        .order_by
        .iter()
        .map(|o| (o, None))
        .chain(nested_order_bys)
        .map(|(order_by, parent)| match order_by {
            OrderBy::Scalar(order_by) => build_order_scalar(ctx, order_by, parent, needs_reversed_order),
            OrderBy::ScalarAggregation(order_by) => build_order_aggr_scalar(order_by, needs_reversed_order),
            OrderBy::ToManyAggregation(order_by) => build_order_aggr_rel(ctx, order_by, parent, needs_reversed_order),
            OrderBy::Relevance(order_by) => build_order_relevance(order_by, needs_reversed_order),
        })
        .collect_vec()
}

fn build_order_scalar(
    ctx: &mut QueryBuilderContext,
    order_by: &OrderByScalar,
    parent: Option<&RelationFieldRef>,
    needs_reversed_order: bool,
) -> OrderByDefinition {
    let (joins, order_column) = compute_joins_scalar(ctx, order_by, parent);

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

fn build_order_relevance(order_by: &OrderByRelevance, needs_reversed_order: bool) -> OrderByDefinition {
    let columns: Vec<Expression> = order_by.fields.iter().map(|sf| sf.as_column().into()).collect();
    let order_column: Expression = text_search_relevance(&columns, order_by.search.clone()).into();
    let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
    let order_definition: OrderDefinition = (order_column.clone(), order);

    OrderByDefinition {
        order_column,
        order_definition,
        joins: vec![],
    }
}

fn build_order_aggr_scalar(order_by: &OrderByScalarAggregation, needs_reversed_order: bool) -> OrderByDefinition {
    let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
    let order_column = order_by.field.as_column();
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
    ctx: &mut QueryBuilderContext,
    order_by: &OrderByToManyAggregation,
    parent: Option<&RelationFieldRef>,
    needs_reversed_order: bool,
) -> OrderByDefinition {
    let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
    let (joins, order_column) = compute_joins_aggregation(ctx, order_by, parent);
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

fn compute_joins_aggregation(
    ctx: &mut QueryBuilderContext,
    order_by: &OrderByToManyAggregation,
    parent: Option<&RelationFieldRef>,
) -> (Vec<AliasedJoin>, Column<'static>) {
    let (last_hop, rest_hops) = order_by
        .path
        .split_last()
        .expect("An order by relation aggregation has to have at least one hop");

    let parent_join = parent.map(|p| ctx.join_builder().compute_one2m_join(p, None));

    // Unwraps are safe because the SQL connector doesn't yet support any other type of orderBy hop but the relation hop.
    let mut joins = vec![];
    for (i, hop) in rest_hops.iter().enumerate() {
        let previous_join = if i > 0 { joins.get(i - 1) } else { None };

        let join = ctx
            .join_builder()
            .compute_one2m_join(hop.as_relation_hop().unwrap(), previous_join);

        joins.push(join);
    }

    let aggregation_type = match order_by.sort_aggregation {
        SortAggregation::Count => AggregationType::Count,
        _ => unreachable!("Order by relation aggregation other than count are not supported"),
    };

    let previous_join = joins.last().or(parent_join.as_ref());

    // We perform the aggregation on the last join
    let (aggr_alias, last_aggr_join) = ctx.join_builder().compute_aggr_join(
        last_hop.as_relation_hop().unwrap(),
        aggregation_type,
        None,
        previous_join,
    );

    // This is the final column identifier to be used for the scalar field to order by.
    // `{last_join_alias}.{ORDER_AGGREGATOR_ALIAS}`
    let order_by_column = Column::from((last_aggr_join.alias.to_owned(), aggr_alias));

    joins.push(last_aggr_join);

    (joins, order_by_column)
}

pub fn compute_joins_scalar(
    ctx: &mut QueryBuilderContext,
    order_by: &OrderByScalar,
    parent: Option<&RelationFieldRef>,
) -> (Vec<AliasedJoin>, Column<'static>) {
    let mut joins = vec![];

    for (i, hop) in order_by.path.iter().enumerate() {
        let previous_join = if i > 0 { joins.get(i - 1) } else { None };
        let join = ctx
            .join_builder()
            .compute_one2m_join(hop.as_relation_hop().unwrap(), previous_join);

        joins.push(join);
    }

    // This is the final column identifier to be used for the scalar field to order by.
    // - If we order by a scalar field on the base model, we simply use the model's scalar field. eg:
    //   `{modelTable}.{field}`
    // - If we order by some relations, we use the alias used for the last join, e.g.
    //   `{join_alias}.{field}`
    let order_by_column = if let Some(last_join) = joins.last() {
        Column::from((last_join.alias.to_owned(), order_by.field.db_name().to_owned()))
    } else if let Some(parent) = parent {
        let join = ctx.join_builder().compute_one2m_join(parent, None);

        Column::from((join.alias.to_owned(), order_by.field.db_name().to_owned()))
    } else {
        order_by.field.as_column()
    };

    (joins, order_by_column)
}

pub fn into_order(prisma_order: &SortOrder, nulls_order: Option<&NullsOrder>, reverse: bool) -> Order {
    match (prisma_order, nulls_order, reverse) {
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

fn process_nested_order_bys(nested_reads: &[NestedRead]) -> Vec<(&OrderBy, Option<&RelationFieldRef>)> {
    let mut normalized = vec![];

    for read in nested_reads {
        for order_by in &read.args.order_by {
            normalized.push((order_by, Some(&read.parent_field)))
        }

        normalized.extend(process_nested_order_bys(&read.nested));
    }

    normalized
}
