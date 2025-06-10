//! Wasm-compatible definitions for the MySQL connector.
//! This module is only available with the `mysql` feature.
mod defaults;
mod error;
#[cfg(feature = "mysql-native")]
pub(crate) mod native;
mod url;

pub use self::url::*;
pub use defaults::*;
pub use error::MysqlError;
