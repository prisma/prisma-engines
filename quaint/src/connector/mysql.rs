pub use wasm::common::MysqlUrl;
pub use wasm::error::MysqlError;

#[cfg(feature = "mysql")]
pub(crate) mod wasm;

#[cfg(feature = "mysql-connector")]
pub(crate) mod native;
