#![allow(clippy::needless_borrow)]

mod engine;
mod error;
mod logger;
mod node_api;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
pub(crate) type Executor = Box<dyn query_core::QueryExecutor + Send + Sync>;
