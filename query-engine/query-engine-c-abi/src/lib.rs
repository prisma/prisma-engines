use query_engine_common::error;

pub mod engine;
pub mod logger;
pub mod migrations;

mod tracer;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
