//! Query graph builder module.

mod builder;
mod error;
mod extractors;

pub(crate) mod read;
pub(crate) mod write;
pub(crate) use extractors::*;

pub use builder::QueryGraphBuilder;
pub use error::*;

/// Query graph builder sub-result type.
pub type QueryGraphBuilderResult<T> = Result<T, QueryGraphBuilderError>;
