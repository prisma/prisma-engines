pub mod engine;
pub mod error;
pub mod functions;
pub mod log_callback;
pub mod logger;

mod nodejs_drivers;
mod tracer;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
pub(crate) type Executor = Box<dyn query_core::QueryExecutor + Send + Sync>;
