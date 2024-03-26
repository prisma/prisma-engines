//! Wasm-compatible definitions for the MSSQL connector.
//! This module is only available with the `mssql` feature.
pub(crate) mod url;

pub use self::url::*;

#[cfg(feature = "mssql-native")]
pub(crate) mod native;
