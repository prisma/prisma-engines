///! Wasm-compatible definitions for the Postgres connector.
/// /// This module is only available with the `postgresql` feature.
pub(crate) mod common;
pub mod error;

pub use common::PostgresUrl;
