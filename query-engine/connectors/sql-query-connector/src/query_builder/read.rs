use crate::{
    cursor_condition, filter::FilterBuilder, model_extensions::*, nested_aggregations, ordering::OrderByBuilder,
    sql_trace::SqlTraceComment, Context,
};
use connector_interface::AggregationSelection;
use itertools::Itertools;
use quaint::ast::*;
use query_structure::*;

pub(crate) trait SelectDefinition {
    fn into_select<'a>(
        self,
        _: &Model,
        virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>);
}

impl SelectDefinition for Filter {
    fn into_select<'a>(
        self,
        model: &Model,
        virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        let args = QueryArguments::from((model.clone(), self));
        args.into_select(model, virtual_selections, ctx)
    }
}

impl SelectDefinition for &Filter {
    fn into_select<'a>(
        self,
        model: &Model,
        virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        self.clone().into_select(model, virtual_selections, ctx)
    }
}

impl SelectDefinition for Select<'static> {
    fn into_select<'a>(
        self,
        _: &Model,
        _: impl IntoIterator<Item = &'a VirtualSelection>,
        _ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        (self, vec![])
    }
}

impl SelectDefinition for QueryArguments {
    fn into_select<'a>(
        self,
        model: &Model,
        virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        let order_by_definitions = OrderByBuilder::default().build(&self, ctx);
        let cursor_condition = cursor_condition::build(&self, model, &order_by_definitions, ctx);
        let aggregation_joins = nested_aggregations::build(virtual_selections, ctx);

        let limit = if self.ignore_take { None } else { self.take_abs() };
        let skip = if self.ignore_skip { 0 } else { self.skip.unwrap_or(0) };

        let (filter, filter_joins) = self
            .filter
            .map(|f| FilterBuilder::with_top_level_joins().visit_filter(f, ctx))
            .unwrap_or((ConditionTree::NoCondition, None));

        let conditions = match (filter, cursor_condition) {
            (ConditionTree::NoCondition, cursor) => cursor,
            (filter, ConditionTree::NoCondition) => filter,
            (filter, cursor) => ConditionTree::and(filter, cursor),
        };

        // Add joins necessary to the ordering
        let joined_table = order_by_definitions
            .iter()
            .flat_map(|j| &j.joins)
            .fold(model.as_table(ctx), |acc, join| acc.join(join.clone().data));

        // Add joins necessary to the nested aggregations
        let joined_table = aggregation_joins
            .joins
            .into_iter()
            .fold(joined_table, |acc, join| acc.join(join.data));

        let joined_table = if let Some(filter_joins) = filter_joins {
            filter_joins
                .into_iter()
                .fold(joined_table, |acc, join| acc.join(join.data))
        } else {
            joined_table
        };

        let select_ast = Select::from_table(joined_table)
            .so_that(conditions)
            .offset(skip as usize)
            .add_traceparent(ctx.traceparent);

        let select_ast = order_by_definitions
            .iter()
            .fold(select_ast, |acc, o| acc.order_by(o.order_definition.clone()));

        let select_ast = if let Some(distinct) = self.distinct {
            let distinct_fields = ModelProjection::from(distinct)
                .as_columns(ctx)
                .map(Expression::from)
                .collect_vec();

            select_ast.distinct_on(distinct_fields)
        } else {
            select_ast
        };

        match limit {
            Some(limit) => (select_ast.limit(limit as usize), aggregation_joins.columns),
            None => (select_ast, aggregation_joins.columns),
        }
    }
}

pub(crate) fn get_records<'a, T>(
    model: &Model,
    columns: impl Iterator<Item = Column<'static>>,
    virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
    query: T,
    ctx: &Context<'_>,
) -> Select<'static>
where
    T: SelectDefinition,
{
    let (select, additional_selection_set) = query.into_select(model, virtual_selections, ctx);
    let select = columns.fold(select, |acc, col| acc.column(col));

    let select = select.add_traceparent(ctx.traceparent);

    additional_selection_set
        .into_iter()
        .fold(select, |acc, col| acc.value(col))
}

/// Generates a query of the form:
/// ```sql
/// SELECT
///     COUNT(*),
///     SUM(`float`),
///     SUM(`int`),
///     AVG(`float`),
///     AVG(`int`),
///     MIN(`float`),
///     MIN(`int`),
///     MAX(`float`),
///     MAX(`int`)
/// FROM
///     (
///         SELECT
///             `Table`.`id`,
///             `Table`.`float`,
///             `Table`.`int`
///         FROM
///             `Table`
///         WHERE
///             1 = 1
///     ) AS `sub`;
/// ```
/// Important note: Do not use the AsColumn trait here as we need to construct column references that are relative,
/// not absolute - e.g. `SELECT "field" FROM (...)` NOT `SELECT "full"."path"."to"."field" FROM (...)`.
pub(crate) fn aggregate(
    model: &Model,
    selections: &[AggregationSelection],
    args: QueryArguments,
    ctx: &Context<'_>,
) -> Select<'static> {
    let columns = extract_columns(model, selections, ctx);
    let sub_query = get_records(model, columns.into_iter(), &[], args, ctx);
    let sub_table = Table::from(sub_query).alias("sub");

    selections.iter().fold(
        Select::from_table(sub_table).add_traceparent(ctx.traceparent),
        |select, next_op| match next_op {
            AggregationSelection::Field(field) => select.column(
                Column::from(field.db_name().to_owned())
                    .set_is_enum(field.type_identifier().is_enum())
                    .set_is_selected(true),
            ),

            AggregationSelection::Count { all, fields } => {
                let select = fields.iter().fold(select, |select, next_field| {
                    select.value(count(Column::from(next_field.db_name().to_owned())))
                });

                if *all {
                    select.value(count(asterisk()))
                } else {
                    select
                }
            }

            AggregationSelection::Average(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(avg(Column::from(next_field.db_name().to_owned())))
            }),

            AggregationSelection::Sum(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(sum(Column::from(next_field.db_name().to_owned())))
            }),

            AggregationSelection::Min(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(min(Column::from(next_field.db_name().to_owned())
                    .set_is_enum(next_field.type_identifier().is_enum())
                    .set_is_selected(true)))
            }),

            AggregationSelection::Max(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(max(Column::from(next_field.db_name().to_owned())
                    .set_is_enum(next_field.type_identifier().is_enum())
                    .set_is_selected(true)))
            }),
        },
    )
}

pub(crate) fn group_by_aggregate(
    model: &Model,
    args: QueryArguments,
    selections: &[AggregationSelection],
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
    ctx: &Context<'_>,
) -> Select<'static> {
    let (base_query, _) = args.into_select(model, &[], ctx);

    let select_query = selections.iter().fold(base_query, |select, next_op| match next_op {
        AggregationSelection::Field(field) => select.column(field.as_column(ctx).set_is_selected(true)),

        AggregationSelection::Count { all, fields } => {
            let select = fields.iter().fold(select, |select, next_field| {
                select.value(count(next_field.as_column(ctx)))
            });

            if *all {
                select.value(count(asterisk()))
            } else {
                select
            }
        }

        AggregationSelection::Average(fields) => fields.iter().fold(select, |select, next_field| {
            select.value(avg(next_field.as_column(ctx)))
        }),

        AggregationSelection::Sum(fields) => fields.iter().fold(select, |select, next_field| {
            select.value(sum(next_field.as_column(ctx)))
        }),

        AggregationSelection::Min(fields) => fields.iter().fold(select, |select, next_field| {
            select.value(min(next_field.as_column(ctx).set_is_selected(true)))
        }),

        AggregationSelection::Max(fields) => fields.iter().fold(select, |select, next_field| {
            select.value(max(next_field.as_column(ctx).set_is_selected(true)))
        }),
    });

    let grouped = group_by
        .into_iter()
        .fold(select_query.add_traceparent(ctx.traceparent), |query, field| {
            query.group_by(field.as_column(ctx))
        });

    match having {
        Some(filter) => {
            let cond = FilterBuilder::without_top_level_joins().visit_filter(filter, ctx);

            grouped.having(cond)
        }
        None => grouped,
    }
}

fn extract_columns(model: &Model, selections: &[AggregationSelection], ctx: &Context<'_>) -> Vec<Column<'static>> {
    let fields: Vec<_> = selections
        .iter()
        .flat_map(|selection| match selection {
            AggregationSelection::Field(field) => vec![field.clone()],
            AggregationSelection::Count { all: _, fields } => {
                if fields.is_empty() {
                    model
                        .primary_identifier()
                        .as_scalar_fields()
                        .expect("Primary identifier has non-scalar fields.")
                } else {
                    fields.clone()
                }
            }
            AggregationSelection::Average(fields) => fields.clone(),
            AggregationSelection::Sum(fields) => fields.clone(),
            AggregationSelection::Min(fields) => fields.clone(),
            AggregationSelection::Max(fields) => fields.clone(),
        })
        .unique_by(|field| field.db_name().to_owned())
        .collect();

    fields.as_columns(ctx).collect()
}
