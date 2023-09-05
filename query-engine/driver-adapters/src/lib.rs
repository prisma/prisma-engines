//! Query Engine Driver Adapters
//! This crate is responsible for defining a quaint::Connector implementation that uses functions
//! exposed by client connectors via N-API.
//!
//! A driver adapter is an object defined in javascript that uses a driver
//! (ex. '@planetscale/database') to provide a similar implementation of that of a quaint Connector. i.e. the ability to query and execute SQL
//! plus some transformation of types to adhere to what a quaint::Value expresses.
//!

mod async_js_function;
mod conversion;
mod error;
mod proxy;
mod queryable;
mod result;
mod transaction;
pub use queryable::{from_napi, JsQueryable};
