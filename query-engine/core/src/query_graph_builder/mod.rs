//! Query graph builder module.
//! tbd

mod error;
mod query_builder;
mod read;
mod utils;

pub mod write;

pub use error::*;
pub use query_builder::*;
pub use read::*;
pub use utils::*;

/// Query graph builder sub-result type.
pub type QueryGraphBuilderResult<T> = Result<T, QueryGraphBuilderError>;

/// Temporary trait for the legacy read builder code.
pub trait Builder<T> {
    fn build(self) -> QueryGraphBuilderResult<T>;
}
