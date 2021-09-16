#![allow(
    clippy::module_inception,
    clippy::wrong_self_convention,
    clippy::vec_init_then_push,
    clippy::upper_case_acronyms,
    clippy::redundant_clone,
    clippy::explicit_counter_loop,
    clippy::match_like_matches_macro,
    clippy::from_over_into,
    clippy::or_fun_call,
    clippy::needless_question_mark,
    clippy::ptr_arg,
    clippy::mem_replace_with_default,
    clippy::clone_on_copy,
    clippy::needless_borrow,
    clippy::needless_collect
)]
#![warn(warnings)] // Todo deny warnings once done

#[macro_use]
extern crate tracing;

pub mod error;
pub mod executor;
pub mod interpreter;
pub mod query_ast;
pub mod query_document;
pub mod query_graph;
pub mod query_graph_builder;
pub mod response_ir;
pub mod result_ast;
pub mod schema;
pub mod schema_builder;

pub use error::*;
pub use executor::*;
pub use interpreter::*;
pub use query_ast::*;
pub use query_document::*;
pub use query_graph::*;
pub use query_graph_builder::*;
pub use response_ir::*;
pub use result_ast::*;
pub use schema::*;
pub use schema_builder::*;

/// Result type tying all sub-result type hierarchies of the core together.
pub type Result<T> = std::result::Result<T, CoreError>;
