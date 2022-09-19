#![deny(warnings)]
#![allow(clippy::derive_partial_eq_without_eq)]

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

pub type Result<T> = std::result::Result<T, error::ConnectorError>;

/// When we write a single record using this update_records function, we always
/// want the id of the changed record back. Even if the row wasn't updated. This can happen in situations where
/// we could increment a null value and the update count would be zero for mysql.
/// However when we updating any records we want to return an empty array if zero items were updated
#[derive(PartialEq)]
pub enum UpdateType {
    Many,
    One,
}
