mod many_related_records;

use crate::{cursor_condition, filter_conversion::AliasedCondition, ordering::Ordering};
use connector_interface::{filter::Filter, QueryArguments};
use prisma_models::*;
use quaint::ast::*;
use std::sync::Arc;

pub use many_related_records::*;

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

        let filter: ConditionTree = self
            .filter
            .map(|f| f.aliased_cond(None))
            .unwrap_or(ConditionTree::NoCondition);

        let conditions = match (filter, cursor) {
            (ConditionTree::NoCondition, cursor) => cursor,
            (filter, ConditionTree::NoCondition) => filter,
            (filter, cursor) => ConditionTree::and(filter, cursor),
        };

        let (skip, limit) = match self.last.or(self.first) {
            Some(c) => (self.skip.unwrap_or(0), Some(c + 1)), // +1 to see if there's more data
            None => (self.skip.unwrap_or(0), None),
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

pub fn count_by_model(_model: &ModelRef, _query_arguments: QueryArguments) -> Select<'static> {
    // let id_field = model.fields().id();

    // let selected_fields = vec![id_field.as_column()];

    // let base_query = get_records(model, selected_fields.into_iter(), query_arguments);
    // let table = Table::from(base_query).alias("sub");
    // let column = Column::from(("sub", id_field.db_name().to_string()));

    // Select::from_table(table).value(count(column))
    todo!()
}

pub fn relation_in_selection<'a, I>(from_field: &RelationFieldRef, identifiers: I) -> ConditionTree<'static>
where
    I: IntoIterator<Item = &'a RecordIdentifier>,
{
    identifiers
        .into_iter()
        .map(|ids| {
            let cols_with_vals = from_field.relation_columns(true).into_iter().zip(ids.values());

            cols_with_vals.fold(ConditionTree::NoCondition, |acc, (col, val)| {
                match acc {
                    ConditionTree::NoCondition => col.equals(val).into(),
                    cond => cond.and(col.equals(val))
                }
            })
        })
        .fold(ConditionTree::NoCondition, |acc, cond| {
            match acc {
                ConditionTree::NoCondition => cond,
                acc => acc.or(cond),
            }
        })
}
