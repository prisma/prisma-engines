#![deny(warnings)]

pub mod error;
pub mod filter;

mod compare;
mod interface;
mod query_arguments;
mod write_args;

pub use compare::*;
pub use filter::*;
pub use interface::*;
pub use query_arguments::*;
pub use write_args::*;

use once_cell::sync::Lazy;
use std::env;

pub type Result<T> = std::result::Result<T, error::ConnectorError>;

/// Number of allowed elements in query's `IN` or `NOT IN` statement.
/// Certain databases error out if querying with too many items. For test
/// purposes, this value can be set with the `QUERY_BATCH_SIZE` environment
/// value to a smaller number.
pub static MAX_BATCH_SIZE: Lazy<usize> = Lazy::new(|| match env::var("QUERY_BATCH_SIZE") {
    Ok(size) => size.parse().unwrap_or(5000),
    Err(_) => 5000,
});
