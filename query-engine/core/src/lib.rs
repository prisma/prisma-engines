#![deny(unsafe_code, rust_2018_idioms)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[macro_use]
extern crate tracing;

pub mod constants;
#[cfg(feature = "executor")]
pub mod executor;
pub mod protocol;
pub mod query_document;
pub mod query_graph_builder;
pub mod relation_load_strategy;
#[cfg(feature = "executor")]
pub mod response_ir;

pub use self::{
    error::{CoreError, ExtendedUserFacingError, FieldConversionError},
    query_ast::*,
    query_document::*,
    query_graph::*,
    query_graph_builder::*,
};

#[cfg(feature = "executor")]
pub use self::{
    executor::{QueryExecutor, TransactionOptions, with_sync_unevaluated_request_context},
    interactive_transactions::{TransactionError, TxId},
};

#[cfg(feature = "executor")]
pub use connector::{
    Connector,
    error::{ConnectorError, ErrorKind as ConnectorErrorKind},
};

mod error;
#[cfg(feature = "executor")]
mod interactive_transactions;
#[cfg(feature = "executor")]
mod interpreter;
#[cfg(feature = "executor")]
mod metrics;
mod query_ast;
mod query_graph;
#[cfg(feature = "executor")]
mod result_ast;

#[cfg(feature = "executor")]
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
