//! Wasm-compatible definitions for the PostgreSQL connector.
//! This module is only available with the `postgresql` feature.
pub(crate) mod error;
pub(crate) mod url;

pub use error::PostgresError;
pub use url::{PostgresFlavour, PostgresUrl};

#[cfg(feature = "postgresql-native")]
pub(crate) mod native;
