//! Query Engine Node.js Driver
//! This crate is responsible for defining a `Queryable` + `TransactionCapable` + `Send` + `Sync` implementation that
//! uses functions exposed by client connectors via N-API.
//!
//! A client connector is an object defined in javascript that uses a driver (ex. '@planetscale/database') to
//! provide a similar implementation of that of a quaint Connector. i.e. the ability to query and execute SQL
//! plus some transformation of types to adhere to what a quaint::Value expresses.
//!

mod proxy;
mod queryable;
pub use queryable::JsQueryable;
