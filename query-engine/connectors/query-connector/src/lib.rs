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
