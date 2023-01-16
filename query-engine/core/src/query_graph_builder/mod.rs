//! Query graph builder module.

mod builder;
mod error;
mod extractors;
mod read;

pub(crate) mod write;

pub(crate) use builder::*;
pub(crate) use error::*;
pub(crate) use extractors::*;

/// Query graph builder sub-result type.
pub type QueryGraphBuilderResult<T> = Result<T, QueryGraphBuilderError>;
