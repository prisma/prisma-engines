//! Wasm-compatible definitions for the MySQL connector.
//! This module is only available with the `mysql` feature.
pub(crate) mod error;
pub(crate) mod url;

pub use error::MysqlError;
pub use url::*;

#[cfg(feature = "mysql-native")]
pub(crate) mod native;
