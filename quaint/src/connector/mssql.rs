pub use wasm::common::MssqlUrl;

#[cfg(feature = "mssql")]
pub(crate) mod wasm;

#[cfg(feature = "mssql-native")]
pub(crate) mod native;
