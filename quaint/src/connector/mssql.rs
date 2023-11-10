pub use wasm::common::MssqlUrl;

#[cfg(feature = "mssql")]
pub(crate) mod wasm;

#[cfg(feature = "mssql-connector")]
pub(crate) mod native;
