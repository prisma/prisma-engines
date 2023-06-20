pub mod engine;
pub mod logger;

#[cfg(not(target_arch = "wasm32"))]
pub mod error;

#[cfg(not(target_arch = "wasm32"))]
pub mod functions;

#[cfg(not(target_arch = "wasm32"))]
pub mod log_callback;

mod tracer;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
pub(crate) type Executor = Box<dyn query_core::QueryExecutor + Send + Sync>;
