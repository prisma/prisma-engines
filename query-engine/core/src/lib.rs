#![warn(warnings)] // todo deny warnings once done

// #[macro_use]
// extern crate log;

#[macro_use]
extern crate debug_stub_derive;

#[macro_use]
extern crate lazy_static;

pub mod error;
pub mod executor;
pub mod interpreter;
pub mod query_ast;
pub mod query_builders;
pub mod query_document;
pub mod query_graph;
pub mod response_ir;
pub mod schema;
pub mod schema_builder;
pub mod result_ast;

pub use error::*;
pub use executor::*;
pub use interpreter::*;
pub use query_ast::*;
pub use query_builders::*;
pub use query_document::*;
pub use query_graph::*;
pub use response_ir::*;
pub use schema::*;
pub use schema_builder::*;
pub use result_ast::*;

pub type CoreResult<T> = Result<T, CoreError>;
