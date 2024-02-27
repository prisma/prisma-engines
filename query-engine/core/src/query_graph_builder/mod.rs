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
use query_structure::{PrismaValue, SelectedField};

/// Query graph builder sub-result type.
pub type QueryGraphBuilderResult<T> = Result<T, QueryGraphBuilderError>;

#[derive(Default, Debug)]
pub struct CompileContext {
    pub fields: HashMap<SelectedField, PrismaValue>,
}
