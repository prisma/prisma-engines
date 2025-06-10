//! Wasm-compatible definitions for the PostgreSQL connector.
//! This module is only available with the `postgresql` feature.
mod defaults;
mod error;
#[cfg(feature = "postgresql-native")]
pub(crate) mod native;
mod url;

pub use self::url::*;
pub use defaults::*;
pub use error::PostgresError;
