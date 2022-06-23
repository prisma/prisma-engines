use crate::{
    cursor_condition, filter_conversion::AliasedCondition, model_extensions::*, nested_aggregations, nested_read,
    ordering, sql_trace::SqlTraceComment,
};
use connector_interface::{filter::Filter, AggregationSelection, NestedRead, QueryArguments, RelAggregationSelection};
use itertools::Itertools;
use prisma_models::*;
use quaint::ast::*;
use tracing::Span;

pub trait SelectDefinition {
    fn into_select(
        self,
        _: &ModelRef,
        nested_reads: &[NestedRead],
        aggr_selections: &[RelAggregationSelection],
        trace_id: Option<String>,
    ) -> (Select<'static>, Vec<Expression<'static>>);
}

impl SelectDefinition for Filter {
    fn into_select(
        self,
        model: &ModelRef,
        nested_reads: &[NestedRead],
        aggr_selections: &[RelAggregationSelection],
        trace_id: Option<String>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        let args = QueryArguments::from((model.clone(), self));
        args.into_select(model, nested_reads, aggr_selections, trace_id)
    }
}

impl SelectDefinition for &Filter {
    fn into_select(
        self,
        model: &ModelRef,
        nested_reads: &[NestedRead],
        aggr_selections: &[RelAggregationSelection],
        trace_id: Option<String>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        self.clone().into_select(model, nested_reads, aggr_selections, trace_id)
    }
}

impl SelectDefinition for Select<'static> {
    fn into_select(
        self,
        _: &ModelRef,
        _: &[NestedRead],
        _: &[RelAggregationSelection],
        _trace_id: Option<String>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        (self, vec![])
    }
}

impl SelectDefinition for QueryArguments {
    fn into_select(
        self,
        model: &ModelRef,
        nested_reads: &[NestedRead],
        aggr_selections: &[RelAggregationSelection],
        trace_id: Option<String>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        let order_by_definitions = OrderByBuilder::default().build(&self);
        let (table_opt, cursor_condition) = cursor_condition::build(&self, &model, &order_by_definitions);
        let aggregation_joins = nested_aggregations::build(aggr_selections);
        let nested_read_joins = nested_read::build_joins(nested_reads);

        let limit = if self.ignore_take { None } else { self.take_abs() };
        let skip = if self.ignore_skip { 0 } else { self.skip.unwrap_or(0) };

        let filter: ConditionTree = self
            .filter
            .map(|f| f.aliased_condition_from(None, false))
            .unwrap_or(ConditionTree::NoCondition);

        let conditions = match (filter, cursor_condition) {
            (ConditionTree::NoCondition, cursor) => cursor,
            (filter, ConditionTree::NoCondition) => filter,
            (filter, cursor) => ConditionTree::and(filter, cursor),
        };

        // Add joins necessary to the ordering
        let joined_table = order_by_definitions
            .iter()
            .flat_map(|j| &j.joins)
            .fold(model.as_table(), |acc, join| acc.left_join(join.clone().data));

        // Add joins necessary to the nested aggregations
        let joined_table = aggregation_joins
            .joins
            .into_iter()
            .fold(joined_table, |acc, join| acc.left_join(join.data));

        let joined_table = nested_read_joins
            .joins
            .into_iter()
            .fold(joined_table, |acc, join| acc.left_join(join.data));

        let select_ast = Select::from_table(joined_table)
            .so_that(conditions)
            .offset(skip as usize)
            .append_trace(&Span::current())
            .add_trace_id(trace_id);

        let select_ast = if let Some(table) = table_opt {
            select_ast.and_from(table)
        } else {
            select_ast
        };

        let select_ast = order_by_definitions
            .iter()
            .fold(select_ast, |acc, o| acc.order_by(o.order_definition.clone()));

        let additional_selection_set = aggregation_joins
            .columns
            .into_iter()
            .chain(nested_read_joins.columns)
            .collect_vec();

        match limit {
            Some(limit) => (select_ast.limit(limit as usize), additional_selection_set),
            None => (select_ast, additional_selection_set),
        }
    }
}

pub fn get_records<T>(
    model: &ModelRef,
    columns: impl Iterator<Item = Column<'static>>,
    aggr_selections: &[RelAggregationSelection],
    query: T,
    nested_reads: &[NestedRead],
    trace_id: Option<String>,
) -> Select<'static>
where
    T: SelectDefinition,
{
    let (select, additional_selection_set) = query.into_select(model, nested_reads, aggr_selections, trace_id.clone());
    let select = columns.fold(select, |acc, col| acc.column(col));

    let select = select.append_trace(&Span::current()).add_trace_id(trace_id);

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
pub fn aggregate(
    model: &ModelRef,
    selections: &[AggregationSelection],
    args: QueryArguments,
    trace_id: Option<String>,
) -> Select<'static> {
    let columns = extract_columns(model, &selections);
    let sub_query = get_records(model, columns.into_iter(), &[], args, &[], trace_id.clone());
    let sub_table = Table::from(sub_query).alias("sub");

    selections.iter().fold(
        Select::from_table(sub_table)
            .append_trace(&Span::current())
            .add_trace_id(trace_id),
        |select, next_op| match next_op {
            AggregationSelection::Field(field) => select.column(Column::from(field.db_name().to_owned())),

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
                select.value(min(Column::from(next_field.db_name().to_owned())))
            }),

            AggregationSelection::Max(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(max(Column::from(next_field.db_name().to_owned())))
            }),
        },
    )
}

pub fn group_by_aggregate(
    model: &ModelRef,
    args: QueryArguments,
    selections: &[AggregationSelection],
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
    trace_id: Option<String>,
) -> Select<'static> {
    let (base_query, _) = args.into_select(model, &[], &[], trace_id.clone());

    let select_query = selections.iter().fold(base_query, |select, next_op| match next_op {
        AggregationSelection::Field(field) => select.column(field.as_column()),

        AggregationSelection::Count { all, fields } => {
            let select = fields
                .iter()
                .fold(select, |select, next_field| select.value(count(next_field.as_column())));

            if *all {
                select.value(count(asterisk()))
            } else {
                select
            }
        }

        AggregationSelection::Average(fields) => fields
            .iter()
            .fold(select, |select, next_field| select.value(avg(next_field.as_column()))),

        AggregationSelection::Sum(fields) => fields
            .iter()
            .fold(select, |select, next_field| select.value(sum(next_field.as_column()))),

        AggregationSelection::Min(fields) => fields
            .iter()
            .fold(select, |select, next_field| select.value(min(next_field.as_column()))),

        AggregationSelection::Max(fields) => fields
            .iter()
            .fold(select, |select, next_field| select.value(max(next_field.as_column()))),
    });

    let grouped = group_by.into_iter().fold(
        select_query.append_trace(&Span::current()).add_trace_id(trace_id),
        |query, field| query.group_by(field.as_column()),
    );

    match having {
        Some(filter) => grouped.having(filter.aliased_condition_from(None, false)),
        None => grouped,
    }
}

fn extract_columns(model: &ModelRef, selections: &[AggregationSelection]) -> Vec<Column<'static>> {
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

    fields.as_columns().collect()
}
