///! Wasm-compatible definitions for the MySQL connector.
/// /// This module is only available with the `mysql` feature.
pub(crate) mod common;
pub mod error;

pub use common::MysqlUrl;
