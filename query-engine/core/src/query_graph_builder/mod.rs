//! Query graph builder module.
//! tbd

mod builder;
mod error;
mod extractors;
mod read;

pub mod write;

pub use builder::*;
pub use error::*;
pub use extractors::*;
pub use read::*;

/// Query graph builder sub-result type.
pub type QueryGraphBuilderResult<T> = Result<T, QueryGraphBuilderError>;

/// Temporary trait for the legacy read builder code.
pub trait Builder<T> {
    fn build(self) -> QueryGraphBuilderResult<T>;
}
