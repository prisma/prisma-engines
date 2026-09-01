use crate::{Context, join_utils::*, model_extensions::*, value::Placeholder};
use itertools::Itertools;
use prisma_value::{Placeholder as PrismaValuePlaceholder, PrismaValue};
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
                OrderBy::ToManyField(order_by) => self.build_order_to_many_field(order_by, needs_reversed_order, ctx),
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

    /// Orders by a specific scalar field on a to-many related model using a correlated subquery.
    ///
    /// Generated SQL:
    /// ```sql
    /// ORDER BY (
    ///   SELECT <related_table>.<field> FROM <related_table>
    ///   WHERE <related_table>.<fk> = <parent_table>.<pk>
    ///   ORDER BY <related_table>.<field> {direction}
    ///   LIMIT 1
    /// ) {direction}
    /// ```
    fn build_order_to_many_field(
        &mut self,
        order_by: &OrderByToManyField,
        needs_reversed_order: bool,
        ctx: &Context<'_>,
    ) -> OrderByDefinition {
        let order: Option<Order> = Some(into_order(
            &order_by.sort_order,
            order_by.nulls_order.as_ref(),
            needs_reversed_order,
        ));

        // The inner subquery uses the original (non-reversed) direction so that LIMIT 1
        // consistently picks the representative value regardless of the outer pagination
        // direction. Only the outer ORDER BY clause needs to respect needs_reversed_order.
        let (intermediary_joins, subquery) =
            self.compute_subquery_for_to_many_field(order_by, ctx);

        let order_definition: OrderDefinition = (subquery.clone(), order);

        OrderByDefinition {
            order_column: subquery,
            order_definition,
            joins: intermediary_joins,
        }
    }

    /// Builds the correlated subquery expression and any intermediary joins for a to-many field ordering.
    fn compute_subquery_for_to_many_field(
        &mut self,
        order_by: &OrderByToManyField,
        ctx: &Context<'_>,
    ) -> (Vec<AliasedJoin>, Expression<'static>) {
        let intermediary_hops = order_by.intermediary_hops();
        let to_many_hop = order_by.to_many_hop().as_relation_hop().unwrap();

        // Build joins for all hops leading up to the to-many relation.
        let parent_alias = self.parent_alias.clone();
        let intermediary_joins = self.compute_one2m_join(intermediary_hops, parent_alias.as_ref(), ctx);

        // The alias for the context table that the correlated subquery references.
        let context_alias = intermediary_joins.last().map(|j| j.alias.clone()).or(parent_alias);

        let subquery = if to_many_hop.relation().is_many_to_many() {
            self.build_m2m_correlated_subquery(to_many_hop, &order_by.field, &order_by.sort_order, order_by.nulls_order.as_ref(), context_alias, ctx)
        } else {
            self.build_one2m_correlated_subquery(to_many_hop, &order_by.field, &order_by.sort_order, order_by.nulls_order.as_ref(), context_alias, ctx)
        };

        (intermediary_joins, subquery)
    }

    /// Builds a correlated sub-SELECT for one-to-many relations:
    /// `(SELECT field FROM Related WHERE Related.fk = Parent.pk ORDER BY field {dir} LIMIT 1)`
    fn build_one2m_correlated_subquery(
        &mut self,
        rf: &RelationFieldRef,
        field: &ScalarFieldRef,
        sort_order: &SortOrder,
        nulls_order: Option<&NullsOrder>,
        context_alias: Option<String>,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        // Alias the inner table so that self-relations don't bind parent-column
        // references to the inner table instead of the outer row.
        let inner_alias = self.join_prefix();

        let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
            // FK is on the parent model side.
            (rf.scalar_fields(), rf.referenced_fields())
        } else {
            // FK is on the related model side.
            (
                rf.related_field().referenced_fields(),
                rf.related_field().scalar_fields(),
            )
        };

        // WHERE right_field (on related/inner table) = left_field (on parent, with alias if applicable)
        let conditions: Vec<Expression<'static>> = left_fields
            .iter()
            .zip(right_fields.iter())
            .map(|(left, right)| {
                let parent_col = left.as_column(ctx).opt_table(context_alias.clone());
                let related_col = right.as_column(ctx).table(inner_alias.clone());
                parent_col.equals(related_col).into()
            })
            .collect();

        let field_col = field.as_column(ctx).table(inner_alias.clone());
        // Use the original (non-reversed) direction so LIMIT 1 always picks the
        // stable representative value for this sort key.
        let inner_order = into_order(sort_order, nulls_order, false);
        let inner_order_def: OrderDefinition<'static> = (field_col.clone().into(), Some(inner_order));

        let subquery = Select::from_table(rf.related_model().as_table(ctx).alias(inner_alias))
            .column(field_col)
            .so_that(ConditionTree::And(conditions))
            .order_by(inner_order_def)
            .limit(1);

        Expression::from(subquery)
    }

    /// Builds a correlated sub-SELECT for many-to-many relations (via junction table):
    /// ```sql
    /// (SELECT field FROM Related
    ///  INNER JOIN _Junction ON Related.id = _Junction.B
    ///  WHERE _Junction.A = Parent.id
    ///  ORDER BY field {dir}
    ///  LIMIT 1)
    /// ```
    fn build_m2m_correlated_subquery(
        &mut self,
        rf: &RelationFieldRef,
        field: &ScalarFieldRef,
        sort_order: &SortOrder,
        nulls_order: Option<&NullsOrder>,
        context_alias: Option<String>,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        // Alias the inner child table so that self-relation M2M subqueries
        // don't confuse inner vs outer column references.
        let inner_alias = self.join_prefix();

        let m2m_table = rf.as_table(ctx);
        // Column in junction that stores parent IDs (used in WHERE for correlation)
        let m2m_parent_col = rf.related_field().m2m_column(ctx);
        // Column in junction that stores child IDs (used in INNER JOIN condition)
        let m2m_child_col = rf.m2m_column(ctx);
        let child_model = rf.related_model();
        let child_ids: ModelProjection = child_model.primary_identifier().into();
        let parent_ids: ModelProjection = rf.model().primary_identifier().into();

        // WHERE _Junction.parent_col = Parent.id (correlated)
        let junction_conditions: Vec<Expression<'static>> = parent_ids
            .scalar_fields()
            .map(|sf| {
                let parent_col = sf.as_column(ctx).opt_table(context_alias.clone());
                let junction_col = m2m_parent_col.clone();
                junction_col.equals(parent_col).into()
            })
            .collect();

        // INNER JOIN _Junction ON Related.id = _Junction.B
        let left_join_conditions: Vec<Expression<'static>> = child_ids
            .as_columns(ctx)
            .map(|c| c.table(inner_alias.clone()).equals(m2m_child_col.clone()).into())
            .collect();

        let field_col = field.as_column(ctx).table(inner_alias.clone());
        // Use the original (non-reversed) direction so LIMIT 1 always picks the
        // stable representative value for this sort key.
        let inner_order = into_order(sort_order, nulls_order, false);
        let inner_order_def: OrderDefinition<'static> = (field_col.clone().into(), Some(inner_order));

        // The WHERE clause already filters to a specific parent via junction_conditions, so
        // the join on the junction table is effectively mandatory — use an inner join.
        let subquery = Select::from_table(child_model.as_table(ctx).alias(inner_alias))
            .column(field_col)
            .so_that(ConditionTree::And(junction_conditions))
            .inner_join(m2m_table.on(ConditionTree::And(left_join_conditions)))
            .order_by(inner_order_def)
            .limit(1);

        Expression::from(subquery)
    }

    fn compute_joins_aggregation(
        &mut self,
        order_by: &OrderByToManyAggregation,
        ctx: &Context<'_>,
    ) -> (Vec<AliasedJoin>, Column<'static>) {
        let intermediary_hops = order_by.intermediary_hops();
        let aggregation_hop = order_by.aggregation_hop();

        // Unwraps are safe because the SQL connector doesn't yet support any other type of orderBy hop but the relation hop.
        let parent_alias = self.parent_alias.clone();
        let mut joins = self.compute_one2m_join(intermediary_hops, parent_alias.as_ref(), ctx);

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
        let search_expr = prisma_value_to_search_expression(order_by.search.clone());
        let text_search_expr = text_search_relevance(&order_by_columns, search_expr);

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

fn prisma_value_to_search_expression(pv: PrismaValue) -> Expression<'static> {
    match pv {
        PrismaValue::String(s) => Value::text(s).into(),
        PrismaValue::Placeholder(PrismaValuePlaceholder { name, .. }) => {
            Value::opaque(Placeholder::new(name), OpaqueType::Text).into()
        }
        _ => panic!("Search field should only contain String or Placeholder values"),
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
