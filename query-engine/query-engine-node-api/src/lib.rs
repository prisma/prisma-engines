pub mod engine;
pub mod error;
pub mod functions;
pub mod logger;
pub(crate) mod response;

mod tracer;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
