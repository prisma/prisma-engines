#![allow(
    clippy::module_inception,
    clippy::vec_init_then_push,
    clippy::derive_partial_eq_without_eq,
    clippy::needless_borrow
)]

#[macro_use]
extern crate tracing;

pub mod executor;
pub mod query_document;
pub mod response_ir;

pub use self::{
    error::{CoreError, FieldConversionError},
    executor::QueryExecutor,
    interactive_transactions::{TransactionError, TxId},
    query_document::*,
    trace_helpers::*,
};

mod error;
mod interactive_transactions;
mod interpreter;
mod query_ast;
mod query_graph;
mod query_graph_builder;
mod result_ast;
mod trace_helpers;

use self::{
    executor::*,
    interactive_transactions::*,
    interpreter::{Env, ExpressionResult, Expressionista, InterpreterError, QueryInterpreter},
    query_ast::*,
    query_graph::*,
    query_graph_builder::*,
    response_ir::{IrSerializer, ResponseData},
    result_ast::*,
};

/// Result type tying all sub-result type hierarchies of the core together.
pub type Result<T> = std::result::Result<T, CoreError>;

// Re-exports
pub extern crate schema;
pub extern crate schema_builder;
