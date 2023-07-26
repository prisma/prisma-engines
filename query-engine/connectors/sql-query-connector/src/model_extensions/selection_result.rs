use crate::context::Context;

use super::{AsColumn, ScalarFieldExt};
use prisma_models::{PrismaValue, SelectedField, SelectionResult};
use quaint::{prelude::Column, Value};

pub(crate) trait SelectionResultExt {
    fn misses_autogen_value(&self) -> bool;
    fn db_values<'a>(&self) -> Vec<Value<'a>>;
    fn columns<'a>(&self, ctx: &Context<'_>) -> Vec<Column<'a>>;

    fn add_autogen_value<V>(&mut self, value: V) -> bool
    where
        V: Into<PrismaValue>;
}

impl SelectionResultExt for SelectionResult {
    fn misses_autogen_value(&self) -> bool {
        self.pairs.iter().any(|p| p.1.is_null())
    }

    fn add_autogen_value<V>(&mut self, value: V) -> bool
    where
        V: Into<PrismaValue>,
    {
        for pair in self.pairs.iter_mut() {
            if pair.1.is_null() {
                pair.1 = value.into();
                return true;
            }
        }

        false
    }

    fn db_values<'a>(&self) -> Vec<Value<'a>> {
        self.pairs
            .iter()
            .map(|(selection, v)| match selection {
                SelectedField::Scalar(sf) => sf.value(v.clone()),
                SelectedField::Composite(_cf) => todo!(), // [Composites] todo
            })
            .collect()
    }

    fn columns<'a>(&self, ctx: &Context<'_>) -> Vec<Column<'a>> {
        self.pairs
            .iter()
            .map(|(field, _)| match field {
                SelectedField::Scalar(sf) => sf.as_column(ctx),
                SelectedField::Composite(_) => todo!(), // [Composites] todo,
            })
            .collect()
    }
}
