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
    let base = args.into_select(model);

    aggregators.into_iter().fold(base, |select, next_op| match next_op {
        Aggregator::Count => select.value(count(asterisk())),

        Aggregator::Average(fields) => fields
            .into_iter()
            .fold(select, |select, next_field| select.value(avg(next_field.as_column()))),

        Aggregator::Sum(fields) => fields
            .into_iter()
            .fold(select, |select, next_field| select.value(sum(next_field.as_column()))),

        Aggregator::Min(fields) => fields
            .into_iter()
            .fold(select, |select, next_field| select.value(min(next_field.as_column()))),

        Aggregator::Max(fields) => fields
            .into_iter()
            .fold(select, |select, next_field| select.value(max(next_field.as_column()))),
    })
}
