use crate::{join_utils::*, model_extensions::*, query_arguments_ext::QueryArgumentsExt};
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

#[derive(Debug, Default)]
pub struct OrderByBuilder {
    // Used to generate unique join alias
    join_counter: usize,
}

impl OrderByBuilder {
    /// Builds all expressions for an `ORDER BY` clause based on the query arguments.
    pub fn build(&mut self, query_arguments: &QueryArguments) -> Vec<OrderByDefinition> {
        let needs_reversed_order = query_arguments.needs_reversed_order();

        // The index is used to differentiate potentially separate relations to the same model.
        query_arguments
            .order_by
            .iter()
            .map(|order_by| match order_by {
                OrderBy::Scalar(order_by) => self.build_order_scalar(order_by, needs_reversed_order),
                OrderBy::ScalarAggregation(order_by) => self.build_order_aggr_scalar(order_by, needs_reversed_order),
                OrderBy::ToManyAggregation(order_by) => self.build_order_aggr_rel(order_by, needs_reversed_order),
                OrderBy::Relevance(order_by) => self.build_order_relevance(order_by, needs_reversed_order),
            })
            .collect_vec()
    }

    fn build_order_scalar(&mut self, order_by: &OrderByScalar, needs_reversed_order: bool) -> OrderByDefinition {
        let (joins, order_column) = self.compute_joins_scalar(order_by);
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

    fn build_order_relevance(&mut self, order_by: &OrderByRelevance, needs_reversed_order: bool) -> OrderByDefinition {
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

    fn build_order_aggr_scalar(
        &mut self,
        order_by: &OrderByScalarAggregation,
        needs_reversed_order: bool,
    ) -> OrderByDefinition {
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
        &mut self,
        order_by: &OrderByToManyAggregation,
        needs_reversed_order: bool,
    ) -> OrderByDefinition {
        let order: Option<Order> = Some(into_order(&order_by.sort_order, None, needs_reversed_order));
        let (joins, order_column) = self.compute_joins_aggregation(order_by);
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
        &mut self,
        order_by: &OrderByToManyAggregation,
    ) -> (Vec<AliasedJoin>, Column<'static>) {
        let (last_hop, rest_hops) = order_by
            .path
            .split_last()
            .expect("An order by relation aggregation has to have at least one hop");

        // Unwraps are safe because the SQL connector doesn't yet support any other type of orderBy hop but the relation hop.
        let mut joins = vec![];
        for (i, hop) in rest_hops.iter().enumerate() {
            let previous_join = if i > 0 { joins.get(i - 1) } else { None };

            let join = compute_one2m_join(hop.into_relation_hop().unwrap(), &self.join_prefix(), previous_join);

            joins.push(join);
        }

        let aggregation_type = match order_by.sort_aggregation {
            SortAggregation::Count => AggregationType::Count,
            _ => unreachable!("Order by relation aggregation other than count are not supported"),
        };

        // We perform the aggregation on the last join
        let last_aggr_join = compute_aggr_join(
            last_hop.into_relation_hop().unwrap(),
            aggregation_type,
            None,
            ORDER_AGGREGATOR_ALIAS,
            &self.join_prefix(),
            joins.last(),
        );

        // This is the final column identifier to be used for the scalar field to order by.
        // `{last_join_alias}.{ORDER_AGGREGATOR_ALIAS}`
        let order_by_column = Column::from((last_aggr_join.alias.to_owned(), ORDER_AGGREGATOR_ALIAS.to_owned()));

        joins.push(last_aggr_join);

        (joins, order_by_column)
    }

    pub fn compute_joins_scalar(&mut self, order_by: &OrderByScalar) -> (Vec<AliasedJoin>, Column<'static>) {
        let mut joins = vec![];

        for (i, hop) in order_by.path.iter().enumerate() {
            let previous_join = if i > 0 { joins.get(i - 1) } else { None };
            let join = compute_one2m_join(hop.into_relation_hop().unwrap(), &self.join_prefix(), previous_join);

            joins.push(join);
        }

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
