use crate::{cursor_condition, filter_conversion::AliasedCondition, ordering::Ordering};
use connector_interface::{filter::Filter, Aggregator, QueryArguments};
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

pub fn aggregate(model: &ModelRef, aggregators: &[Aggregator], args: QueryArguments) -> Select<'static> {
    let columns = extract_columns(model, &aggregators);
    let sub_query = get_records(model, columns.into_iter(), args);
    let sub_table = Table::from(sub_query).alias("sub");

    aggregators
        .into_iter()
        .fold(Select::from_table(sub_table), |select, next_op| match next_op {
            Aggregator::Count => select.value(count(asterisk())),

            Aggregator::Average(fields) => fields.into_iter().fold(select, |select, next_field| {
                select.value(avg(col_alias("avg", &next_field.name)))
            }),

            Aggregator::Sum(fields) => fields.into_iter().fold(select, |select, next_field| {
                select.value(sum(col_alias("sum", &next_field.name)))
            }),

            Aggregator::Min(fields) => fields.into_iter().fold(select, |select, next_field| {
                select.value(min(col_alias("min", &next_field.name)))
            }),

            Aggregator::Max(fields) => fields.into_iter().fold(select, |select, next_field| {
                select.value(max(col_alias("max", &next_field.name)))
            }),
        })
}

fn extract_columns(model: &ModelRef, aggregators: &[Aggregator]) -> Vec<Column<'static>> {
    aggregators
        .iter()
        .flat_map(|aggregator| match aggregator {
            Aggregator::Count => model.primary_identifier().as_columns().collect(),
            Aggregator::Average(fields) => map_aggregator_field_columns("avg", fields),
            Aggregator::Sum(fields) => map_aggregator_field_columns("sum", fields),
            Aggregator::Min(fields) => map_aggregator_field_columns("min", fields),
            Aggregator::Max(fields) => map_aggregator_field_columns("max", fields),
        })
        .collect()
}

fn map_aggregator_field_columns(prefix: &str, fields: &[ScalarFieldRef]) -> Vec<Column<'static>> {
    fields
        .into_iter()
        .map(|f| {
            let col = f.as_column();
            col.alias(col_alias(prefix, &f.name))
        })
        .collect()
}

fn col_alias(prefix: &str, field_name: &str) -> String {
    format!("{}_{}", prefix, field_name)
}
