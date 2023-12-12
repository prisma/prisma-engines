mod wasm;

pub use wasm::*;
pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
