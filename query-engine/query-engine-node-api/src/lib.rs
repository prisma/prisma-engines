pub mod engine;
pub mod error;
pub mod functions;
pub mod logger;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
