//! Query Engine Client-Side Driver
//!
//! This crate is responsible for defining a `Queryable` + `TransactionCapable` + `Send` + `Sync` implementation that
//! uses an external driver provided by the client and exposed via FFI (or even RPC).
//!
//! For the [`Driver`] implementation which uses functions exposed by Node.js drivers via N-API,
//! see the `nodejs_drivers` module in the `query-engine-node-api` crate.

mod driver;
mod queryable;

pub use driver::{Driver, Error, Result, ResultSet};
pub use queryable::install_driver;
pub use queryable::installed_driver;
pub use queryable::Queryable;
