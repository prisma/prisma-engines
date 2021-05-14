#![deny(warnings)]

pub mod error;
pub mod filter;

mod coerce;
mod compare;
mod interface;
mod query_arguments;
mod write_args;

pub use coerce::*;
pub use compare::*;
pub use filter::*;
pub use interface::*;
pub use query_arguments::*;
pub use write_args::*;

use enumflags2::BitFlags;
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

/// Flags to setup the connector behavior outside of the connection string.
#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum SourceParameter {
    /// Enable logging of the query SQL and parameters.
    QueryLogging = 1 << 0,
}
