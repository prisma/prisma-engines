//! Query graph builder module.

mod builder;
mod error;
mod extractors;
mod read;

pub(crate) mod write;

pub(crate) use error::*;
pub(crate) use extractors::*;

pub use builder::QueryGraphBuilder;

/// Query graph builder sub-result type.
pub type QueryGraphBuilderResult<T> = Result<T, QueryGraphBuilderError>;
