//! Wasm-compatible definitions for the PostgreSQL connector.
//! This module is only available with the `postgresql` feature.
pub(crate) mod error;
pub(crate) mod url;

pub use self::url::*;
pub use error::PostgresError;

#[cfg(feature = "postgresql-native")]
pub(crate) mod native;
