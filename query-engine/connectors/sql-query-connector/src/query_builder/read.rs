use crate::{cursor_condition, filter_conversion::AliasedCondition, ordering::Ordering};
use connector_interface::{filter::Filter, Aggregator, QueryArguments};
use itertools::Itertools;
use prisma_models::*;
use quaint::ast::*;
use std::sync::Arc;

pub trait SelectDefinition {
    fn into_select(self, _: &ModelRef) -> Select<'static>;
}

impl SelectDefinition for Filter {
    fn into_select(self, model: &ModelRef) -> Select<'static> {
        let args = QueryArguments::from(self);
        args.into_select(model)
    }
}

impl SelectDefinition for &Filter {
    fn into_select(self, model: &ModelRef) -> Select<'static> {
        self.clone().into_select(model)
    }
}

impl SelectDefinition for Select<'static> {
    fn into_select(self, _: &ModelRef) -> Select<'static> {
        self
    }
}

impl SelectDefinition for QueryArguments {
    fn into_select(self, model: &ModelRef) -> Select<'static> {
        let cursor: ConditionTree = cursor_condition::build(&self, Arc::clone(&model));
        let ordering_directions = self.ordering_directions();
        let ordering = Ordering::for_model(&model, ordering_directions);

        let limit = if self.ignore_take { None } else { self.take_abs() };
        let skip = if self.ignore_skip { 0 } else { self.skip.unwrap_or(0) };

        let filter: ConditionTree = self
            .filter
            .map(|f| f.aliased_cond(None))
            .unwrap_or(ConditionTree::NoCondition);

        let conditions = match (filter, cursor) {
            (ConditionTree::NoCondition, cursor) => cursor,
            (filter, ConditionTree::NoCondition) => filter,
            (filter, cursor) => ConditionTree::and(filter, cursor),
        };

        let select_ast = Select::from_table(model.as_table())
            .so_that(conditions)
            .offset(skip as usize);

        let select_ast = ordering.into_iter().fold(select_ast, |acc, ord| acc.order_by(ord));

        match limit {
            Some(limit) => select_ast.limit(limit as usize),
            None => select_ast,
        }
    }
}

pub fn get_records<T>(model: &ModelRef, columns: impl Iterator<Item = Column<'static>>, query: T) -> Select<'static>
where
    T: SelectDefinition,
{
    columns.fold(query.into_select(model), |acc, col| acc.column(col))
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
pub fn aggregate(model: &ModelRef, aggregators: &[Aggregator], args: QueryArguments) -> Select<'static> {
    let columns = extract_columns(model, &aggregators);
    let sub_query = get_records(model, columns.into_iter(), args);
    let sub_table = Table::from(sub_query).alias("sub");

    aggregators
        .into_iter()
        .fold(Select::from_table(sub_table), |select, next_op| match next_op {
            Aggregator::Count => select.value(count(asterisk())),

            Aggregator::Average(fields) => fields
                .into_iter()
                .fold(select, |select, next_field| select.value(avg(next_field.name.clone()))),

            Aggregator::Sum(fields) => fields
                .into_iter()
                .fold(select, |select, next_field| select.value(sum(next_field.name.clone()))),

            Aggregator::Min(fields) => fields
                .into_iter()
                .fold(select, |select, next_field| select.value(min(next_field.name.clone()))),

            Aggregator::Max(fields) => fields
                .into_iter()
                .fold(select, |select, next_field| select.value(max(next_field.name.clone()))),
        })
}

fn extract_columns(model: &ModelRef, aggregators: &[Aggregator]) -> Vec<Column<'static>> {
    let fields: Vec<_> = aggregators
        .iter()
        .flat_map(|aggregator| match aggregator {
            Aggregator::Count => model.primary_identifier().scalar_fields().collect(),
            Aggregator::Average(fields) => fields.clone(),
            Aggregator::Sum(fields) => fields.clone(),
            Aggregator::Min(fields) => fields.clone(),
            Aggregator::Max(fields) => fields.clone(),
        })
        .unique_by(|field| field.name.clone())
        .collect();

    fields.as_columns().collect()
}
