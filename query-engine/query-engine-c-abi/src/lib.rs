pub mod engine;
pub mod error;
pub mod functions;
pub mod logger;
pub mod migrations;

mod tracer;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
