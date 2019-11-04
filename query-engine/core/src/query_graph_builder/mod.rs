//! Query graph builder module.
//! tbd

mod builder;
mod error;
mod read;
mod utils;

pub mod write;

pub use builder::*;
pub use error::*;
pub use read::*;
pub use utils::*;

/// Query graph builder sub-result type.
pub type QueryGraphBuilderResult<T> = Result<T, QueryGraphBuilderError>;

/// Temporary trait for the legacy read builder code.
pub trait Builder<T> {
    fn build(self) -> QueryGraphBuilderResult<T>;
}
