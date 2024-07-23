//! Wasm-compatible definitions for the SQLite connector.
//! This module is only available with the `sqlite` feature.
mod defaults;

pub(crate) mod error;
mod ffi;
pub(crate) mod params;

pub use defaults::*;
pub use error::SqliteError;
pub use params::*;

#[cfg(feature = "sqlite-native")]
pub(crate) mod native;
