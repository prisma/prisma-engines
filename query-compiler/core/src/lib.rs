#![deny(unsafe_code, rust_2018_idioms)]

pub mod constants;
pub mod protocol;
pub mod query_document;
pub mod query_graph_builder;
pub mod relation_load_strategy;
pub mod request_context;

pub use self::{
    error::{CoreError, ExtendedUserFacingError},
    query_ast::*,
    query_document::*,
    query_graph::*,
    query_graph_builder::*,
    request_context::with_sync_unevaluated_request_context,
};

mod error;
mod query_ast;
mod query_graph;

/// Result type tying all sub-result type hierarchies of the core together.
pub type Result<T> = std::result::Result<T, CoreError>;

// Re-exports
pub use schema;
