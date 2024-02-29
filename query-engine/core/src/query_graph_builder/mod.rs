//! Query graph builder module.

mod builder;
mod error;
mod extractors;
mod read;

pub(crate) mod write;
use std::collections::HashMap;

pub(crate) use extractors::*;

pub use builder::QueryGraphBuilder;
pub use error::*;
use query_structure::{FieldSelection, PrismaValue, SelectedField, SelectionResult};

/// Query graph builder sub-result type.
pub type QueryGraphBuilderResult<T> = Result<T, QueryGraphBuilderError>;

// TODO laplab: comment.
#[derive(Default, Debug)]
pub struct CompileContext {
    fields: HashMap<SelectedField, PrismaValue>,
}

impl CompileContext {
    pub fn contains(&self, selection: &FieldSelection) -> bool {
        for field in selection.selections() {
            if !self.fields.contains_key(field) {
                return false;
            }
        }

        true
    }

    pub fn lookup(&self, selection: FieldSelection) -> Option<SelectionResult> {
        let mut values = vec![];
        for field in selection.into_inner() {
            match self.fields.get(&field) {
                Some(value) => values.push((field, value.clone())),
                None => return None,
            }
        }

        Some(SelectionResult::new(values))
    }

    pub fn insert(&mut self, field: SelectedField, value: PrismaValue) {
        self.fields.insert(field, value);
    }
}
