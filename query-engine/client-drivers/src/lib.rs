//! Query Engine Node.js Driver
//! This crate is responsible for defining a `Queryable` + `TransactionCapable` + `Send` + `Sync` implementation that
//! uses functions exposed by Node.js drivers via N-API.
//!

mod driver;
mod queryable;

pub use driver::{Driver, Error, Result, ResultSet};
pub use queryable::install_driver;
pub use queryable::installed_driver;
pub use queryable::Queryable;
