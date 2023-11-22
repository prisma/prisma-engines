pub mod engine;
pub mod error;
pub mod functions;
pub mod logger;

pub(crate) type Executor = Box<dyn query_core::QueryExecutor + Send + Sync>;
