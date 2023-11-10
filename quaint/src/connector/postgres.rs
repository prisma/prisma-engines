pub use wasm::common::PostgresUrl;
pub use wasm::error::PostgresError;

#[cfg(feature = "postgresql")]
pub(crate) mod wasm;

#[cfg(feature = "postgresql-connector")]
pub(crate) mod native;
