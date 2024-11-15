//! Wasm-compatible definitions for the MySQL connector.
//! This module is only available with the `mysql` feature.
mod defaults;

pub(crate) mod error;
pub(crate) mod url;

pub use self::url::*;
pub use error::MysqlError;

pub use defaults::*;
#[cfg(feature = "mysql-native")]
pub(crate) mod native;
