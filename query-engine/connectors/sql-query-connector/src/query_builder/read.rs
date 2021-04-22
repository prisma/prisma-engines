use crate::{cursor_condition, filter_conversion::AliasedCondition, nested_aggregations, ordering};
use connector_interface::{filter::Filter, AggregationSelection, QueryArguments, RelAggregationSelection};
use itertools::Itertools;
use prisma_models::*;
use quaint::ast::*;

pub trait SelectDefinition {
    fn into_select(
        self,
        _: &ModelRef,
        aggr_selections: &[RelAggregationSelection],
    ) -> (Select<'static>, Vec<Column<'static>>);
}

impl SelectDefinition for Filter {
    fn into_select(
        self,
        model: &ModelRef,
        aggr_selections: &[RelAggregationSelection],
    ) -> (Select<'static>, Vec<Column<'static>>) {
        let args = QueryArguments::from((model.clone(), self));
        args.into_select(model, aggr_selections)
    }
}

impl SelectDefinition for &Filter {
    fn into_select(
        self,
        model: &ModelRef,
        aggr_selections: &[RelAggregationSelection],
    ) -> (Select<'static>, Vec<Column<'static>>) {
        self.clone().into_select(model, aggr_selections)
    }
}

impl SelectDefinition for Select<'static> {
    fn into_select(self, _: &ModelRef, _: &[RelAggregationSelection]) -> (Select<'static>, Vec<Column<'static>>) {
        (self, vec![])
    }
}

impl SelectDefinition for QueryArguments {
    #[tracing::instrument(skip(self, model, aggr_selections))]
    fn into_select(
        self,
        model: &ModelRef,
        aggr_selections: &[RelAggregationSelection],
    ) -> (Select<'static>, Vec<Column<'static>>) {
        let (orderings, ordering_joins) = ordering::build(&self, &model);
        let (table_opt, cursor_condition) = cursor_condition::build(&self, &model, &ordering_joins);
        let aggregation_joins = nested_aggregations::build(aggr_selections);

        let limit = if self.ignore_take { None } else { self.take_abs() };
        let skip = if self.ignore_skip { 0 } else { self.skip.unwrap_or(0) };

        let filter: ConditionTree = self
            .filter
            .map(|f| f.aliased_cond(None))
            .unwrap_or(ConditionTree::NoCondition);

        let conditions = match (filter, cursor_condition) {
            (ConditionTree::NoCondition, cursor) => cursor,
            (filter, ConditionTree::NoCondition) => filter,
            (filter, cursor) => ConditionTree::and(filter, cursor),
        };

        // Add joins necessary to the ordering
        let joined_table = ordering_joins
            .into_iter()
            .flat_map(|j| j.joins)
            .fold(model.as_table(), |acc, join| acc.left_join(join.data));

        // Add joins necessary to the nested aggregations
        let joined_table = aggregation_joins
            .joins
            .into_iter()
            .fold(joined_table, |acc, join| acc.left_join(join.data));

        let select_ast = Select::from_table(joined_table)
            .so_that(conditions)
            .offset(skip as usize);

        let select_ast = if let Some(table) = table_opt {
            select_ast.and_from(table)
        } else {
            select_ast
        };

        let select_ast = orderings.into_iter().fold(select_ast, |acc, ord| acc.order_by(ord));

        match limit {
            Some(limit) => (select_ast.limit(limit as usize), aggregation_joins.columns),
            None => (select_ast, aggregation_joins.columns),
        }
    }
}

#[tracing::instrument(skip(model, columns, aggr_selections, query))]
pub fn get_records<T>(
    model: &ModelRef,
    columns: impl Iterator<Item = Column<'static>>,
    aggr_selections: &[RelAggregationSelection],
    query: T,
) -> Select<'static>
where
    T: SelectDefinition,
{
    let (select, aggr_columns) = query.into_select(model, aggr_selections);
    let select = columns.fold(select, |acc, col| acc.column(col));

    aggr_columns.into_iter().fold(select, |acc, col| acc.column(col))
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
#[tracing::instrument(skip(model, selections, args))]
pub fn aggregate(model: &ModelRef, selections: &[AggregationSelection], args: QueryArguments) -> Select<'static> {
    let columns = extract_columns(model, &selections);
    let sub_query = get_records(model, columns.into_iter(), &[], args);
    let sub_table = Table::from(sub_query).alias("sub");

    selections
        .iter()
        .fold(Select::from_table(sub_table), |select, next_op| match next_op {
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
        })
}

#[tracing::instrument(skip(model, args, selections, group_by, having))]
pub fn group_by_aggregate(
    model: &ModelRef,
    args: QueryArguments,
    selections: &[AggregationSelection],
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
) -> Select<'static> {
    let (base_query, _) = args.into_select(model, &[]);

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

    let grouped = group_by
        .into_iter()
        .fold(select_query, |query, field| query.group_by(field.as_column()));

    match having {
        Some(filter) => grouped.having(filter.aliased_cond(None)),
        None => grouped,
    }
}

#[tracing::instrument(skip(model, selections))]
fn extract_columns(model: &ModelRef, selections: &[AggregationSelection]) -> Vec<Column<'static>> {
    let fields: Vec<_> = selections
        .iter()
        .flat_map(|selection| match selection {
            AggregationSelection::Field(field) => vec![field.clone()],
            AggregationSelection::Count { all: _, fields } => {
                if fields.is_empty() {
                    model.primary_identifier().scalar_fields().collect()
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
