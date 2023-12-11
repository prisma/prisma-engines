//! Common definitions and functions for the Query Engine library.

pub mod engine;
pub mod error;
pub mod logger;
pub mod tracer;

pub type Result<T> = std::result::Result<T, error::ApiError>;
pub type Executor = Box<dyn query_core::QueryExecutor + Send + Sync>;
