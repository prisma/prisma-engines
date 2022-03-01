use super::ScalarFieldExt;
use prisma_models::{PrismaValue, SelectedField, SelectionResult};
use quaint::Value;

pub trait SelectionResultExt {
    fn misses_autogen_value(&self) -> bool;
    fn db_values<'a>(&self) -> Vec<Value<'a>>;

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
}
