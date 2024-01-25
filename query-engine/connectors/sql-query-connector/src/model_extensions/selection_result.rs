use super::ScalarFieldExt;
use crate::context::Context;
use quaint::Value;
use query_structure::{PrismaValue, SelectedField, SelectionResult};

pub(crate) trait SelectionResultExt {
    fn misses_autogen_value(&self) -> bool;
    fn db_values<'a>(&self, ctx: &Context<'_>) -> Vec<Value<'a>>;

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

    fn db_values<'a>(&self, ctx: &Context<'_>) -> Vec<Value<'a>> {
        self.pairs
            .iter()
            .filter_map(|(selection, v)| match selection {
                SelectedField::Scalar(sf) => Some(sf.value(v.clone(), ctx)),
                SelectedField::Composite(_) => None,
                SelectedField::Relation(_) => None,
                SelectedField::Virtual(_) => None,
            })
            .collect()
    }
}
