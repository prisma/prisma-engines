#![allow(clippy::needless_borrow)]

pub mod engine;
pub mod error;
pub mod logger;
pub mod node_api;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
pub(crate) type Executor = Box<dyn query_core::QueryExecutor + Send + Sync>;
