use crate::{join_utils::*, model_extensions::*, Context};
use itertools::Itertools;
use psl::{datamodel_connector::ConnectorCapability, reachable_only_with_capability};
use quaint::ast::*;
use query_builder::QueryArgumentsExt;
use query_structure::*;

static ORDER_JOIN_PREFIX: &str = "orderby_";
static ORDER_AGGREGATOR_ALIAS: &str = "orderby_aggregator";

#[derive(Debug, Clone)]
pub(crate) struct OrderByDefinition {
    /// Final column identifier to be used for the scalar field to order by
    pub(crate) order_column: Expression<'static>,
    /// Defines ordering for an `ORDER BY` statement.
    pub(crate) order_definition: OrderDefinition<'static>,
    /// Joins necessary to perform the order by
    pub(crate) joins: Vec<AliasedJoin>,
}

#[derive(Debug, Default)]
pub(crate) struct OrderByBuilder {
    /// Parent table alias, used mostly for relationLoadStrategy: join when performing nested ordering.
    /// This parent alias enables us to prefix the ordered field with the correct parent join alias.
    parent_alias: Option<String>,
    /// Counter used to generate unique join alias
    join_counter: usize,
}

impl OrderByBuilder {
    #[cfg(feature = "relation_joins")]
    pub(crate) fn with_parent_alias(mut self, alias: Option<String>) -> Self {
        self.parent_alias = alias;
        self
    }
}

impl OrderByBuilder {
    /// Builds all expressions for an `ORDER BY` clause based on the query arguments.
    pub(crate) fn build(&mut self, query_arguments: &QueryArguments, ctx: &Context<'_>) -> Vec<OrderByDefinition> {
        let needs_reversed_order = query_arguments.needs_reversed_order();

        query_arguments
            .order_by
            .iter()
            .map(|order_by| match order_by {
                OrderBy::Scalar(order_by) => self.build_order_scalar(order_by, needs_reversed_order, ctx),
                OrderBy::ScalarAggregation(order_by) => {
                    self.build_order_aggr_scalar(order_by, needs_reversed_order, ctx)
                }
                OrderBy::ToManyAggregation(order_by) => self.build_order_aggr_rel(order_by, needs_reversed_order, ctx),
                OrderBy::Relevance(order_by) => {
                    reachable_only_with_capability!(ConnectorCapability::NativeFullTextSearch);
                    self.build_order_relevance(order_by, needs_reversed_order, ctx)
                }
            })
            .collect_vec()
    }

    fn build_order_scalar(
        &mut self,
        order_by: &OrderByScalar,
        needs_reversed_order: bool,
        ctx: &Context<'_>,
    ) -> OrderByDefinition {
        let (joins, order_column) = self.compute_joins_scalar(order_by, ctx);
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

    fn build_order_relevance(
        &mut self,
        order_by: &OrderByRelevance,
        needs_reversed_order: bool,
        ctx: &Context<'_>,
    ) -> OrderByDefinition {
        let (joins, order_column) = self.compute_joins_relevance(order_by, ctx);
        let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
        let order_definition: OrderDefinition = (order_column.clone(), order);

        OrderByDefinition {
            order_column,
            order_definition,
            joins,
        }
    }

    fn build_order_aggr_scalar(
        &mut self,
        order_by: &OrderByScalarAggregation,
        needs_reversed_order: bool,
        ctx: &Context<'_>,
    ) -> OrderByDefinition {
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

    fn build_order_aggr_rel(
        &mut self,
        order_by: &OrderByToManyAggregation,
        needs_reversed_order: bool,
        ctx: &Context<'_>,
    ) -> OrderByDefinition {
        let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
        let (joins, order_column) = self.compute_joins_aggregation(order_by, ctx);
        let order_definition: OrderDefinition = match order_by.sort_aggregation {
            SortAggregation::Count => {
                let exprs: Vec<Expression> = vec![order_column.clone().into(), Value::int32(0).into()];

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
        &mut self,
        order_by: &OrderByToManyAggregation,
        ctx: &Context<'_>,
    ) -> (Vec<AliasedJoin>, Column<'static>) {
        let intermediary_hops = order_by.intermediary_hops();
        let aggregation_hop = order_by.aggregation_hop();

        // Unwraps are safe because the SQL connector doesn't yet support any other type of orderBy hop but the relation hop.
        let mut joins: Vec<AliasedJoin> = vec![];

        let parent_alias = self.parent_alias.clone();

        for (i, hop) in intermediary_hops.iter().enumerate() {
            let previous_join = if i > 0 { joins.get(i - 1) } else { None };

            let previous_alias = previous_join.map(|j| j.alias.as_str()).or(parent_alias.as_deref());
            let join = compute_one2m_join(hop.as_relation_hop().unwrap(), &self.join_prefix(), previous_alias, ctx);

            joins.push(join);
        }

        let aggregation_type = match order_by.sort_aggregation {
            SortAggregation::Count => AggregationType::Count,
            _ => unreachable!("Order by relation aggregation other than count are not supported"),
        };

        let previous_alias = joins.last().map(|j| j.alias.as_str()).or(parent_alias.as_deref());

        // We perform the aggregation on the last join
        let last_aggr_join = compute_aggr_join(
            aggregation_hop.as_relation_hop().unwrap(),
            aggregation_type,
            None,
            ORDER_AGGREGATOR_ALIAS,
            &self.join_prefix(),
            previous_alias,
            ctx,
        );

        // This is the final column identifier to be used for the scalar field to order by.
        // `{last_join_alias}.{ORDER_AGGREGATOR_ALIAS}`
        let order_by_column = Column::from((last_aggr_join.alias.to_owned(), ORDER_AGGREGATOR_ALIAS.to_owned()));

        joins.push(last_aggr_join);

        (joins, order_by_column)
    }

    pub(crate) fn compute_joins_scalar(
        &mut self,
        order_by: &OrderByScalar,
        ctx: &Context<'_>,
    ) -> (Vec<AliasedJoin>, Column<'static>) {
        let parent_alias = self.parent_alias.clone();
        let joins: Vec<AliasedJoin> = self.compute_one2m_join(&order_by.path, parent_alias.as_ref(), ctx);

        // This is the final column identifier to be used for the scalar field to order by.
        // - If we order by a scalar field on the base model, we simply use the model's scalar field. eg:
        //   `{modelTable}.{field}`
        // - If there's a parent_alias, we use it to prefix the field, e.g. `{parent_alias}.{field}`
        // - If we order by some relations, we use the alias used for the last join, e.g.
        //   `{join_alias}.{field}`
        let parent_table = joins
            .last()
            .map(|j| j.alias.to_owned())
            .or_else(|| self.parent_alias.clone());
        let order_by_column = order_by.field.as_column(ctx).opt_table(parent_table);

        (joins, order_by_column)
    }

    pub(crate) fn compute_joins_relevance(
        &mut self,
        order_by: &OrderByRelevance,
        ctx: &Context<'_>,
    ) -> (Vec<AliasedJoin>, Expression<'static>) {
        let parent_alias = self.parent_alias.clone();
        let joins: Vec<AliasedJoin> = self.compute_one2m_join(&order_by.path, parent_alias.as_ref(), ctx);

        // This is the final column identifier to be used for the scalar field to order by.
        // - If we order by a scalar field on the base model, we simply use the model's scalar field. eg:
        //   `{modelTable}.{field}`
        // - If there's a parent_alias, we use it to prefix the field, e.g. `{parent_alias}.{field}`
        // - If we order by some relations, we use the alias used for the last join, e.g.
        //   `{join_alias}.{field}`
        let parent_table = joins
            .last()
            .map(|j| j.alias.to_owned())
            .or_else(|| self.parent_alias.clone());
        let order_by_columns: Vec<_> = order_by
            .fields
            .iter()
            .map(|sf| sf.as_column(ctx).opt_table(parent_table.clone()))
            .map(Expression::from)
            .collect();
        let text_search_expr = text_search_relevance(&order_by_columns, order_by.search.clone());

        (joins, text_search_expr.into())
    }

    fn compute_one2m_join(
        &mut self,
        path: &[OrderByHop],
        parent_alias: Option<&String>,
        ctx: &Context<'_>,
    ) -> Vec<AliasedJoin> {
        let mut joins: Vec<AliasedJoin> = vec![];

        for (i, hop) in path.iter().enumerate() {
            let previous_join = if i > 0 { joins.get(i - 1) } else { None };
            let previous_alias = previous_join
                .map(|j| &j.alias)
                .or(parent_alias)
                .map(|alias| alias.as_str());
            let join = crate::join_utils::compute_one2m_join(
                hop.as_relation_hop().unwrap(),
                &self.join_prefix(),
                previous_alias,
                ctx,
            );

            joins.push(join);
        }

        joins
    }

    fn join_prefix(&mut self) -> String {
        self.join_counter += 1;

        format!("{}{}", ORDER_JOIN_PREFIX, self.join_counter)
    }
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
