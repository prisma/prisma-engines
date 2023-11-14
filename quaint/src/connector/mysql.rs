pub use wasm::common::MysqlUrl;
pub use wasm::error::MysqlError;

#[cfg(feature = "mysql")]
pub(crate) mod wasm;

#[cfg(feature = "mysql-native")]
pub(crate) mod native;
