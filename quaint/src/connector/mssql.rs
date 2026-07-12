//! Wasm-compatible definitions for the MSSQL connector.
//! This module is only available with the `mssql` feature.
mod defaults;
mod error;
#[cfg(feature = "mssql-native")]
pub(crate) mod native;
mod url;

pub use self::url::*;
pub use defaults::*;
pub use error::MssqlError;
