pub use wasm::error::SqliteError;

#[cfg(feature = "sqlite")]
pub(crate) mod wasm;

#[cfg(feature = "sqlite-connector")]
pub(crate) mod native;
