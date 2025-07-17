#![deny(unsafe_code, rust_2018_idioms)]

#[macro_use]
extern crate tracing;

pub mod constants;
pub mod executor;
pub mod protocol;
pub mod query_document;
pub mod query_graph_builder;
pub mod relation_load_strategy;
pub mod response_ir;

pub use self::{
    error::{CoreError, ExtendedUserFacingError, FieldConversionError},
    executor::{QueryExecutor, TransactionOptions, with_sync_unevaluated_request_context},
    interactive_transactions::{TransactionError, TxId},
    query_ast::*,
    query_document::*,
    query_graph::*,
    query_graph_builder::*,
};

pub use connector::{
    Connector,
    error::{ConnectorError, ErrorKind as ConnectorErrorKind},
};

mod error;
mod interactive_transactions;
mod interpreter;
mod metrics;
mod query_ast;
mod query_graph;
mod result_ast;

use self::{
    executor::*,
    interactive_transactions::*,
    interpreter::{Env, ExpressionResult, Expressionista, InterpreterError, QueryInterpreter},
    response_ir::{IrSerializer, ResponseData},
    result_ast::*,
};

/// Result type tying all sub-result type hierarchies of the core together.
pub type Result<T> = std::result::Result<T, CoreError>;

// Re-exports
pub use schema;
