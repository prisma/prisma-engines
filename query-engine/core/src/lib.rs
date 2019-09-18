#![warn(warnings)] // todo deny warnings once done

// #[macro_use]
// extern crate log;

#[macro_use]
extern crate debug_stub_derive;

#[macro_use]
extern crate lazy_static;

pub mod error;
pub mod executor;
pub mod query_builders;
pub mod query_document;
pub mod query_graph;
pub mod response_ir;
pub mod schema;
pub mod schema_builder;
pub mod query_ast;
pub mod interpreter;

pub use query_graph::*;
pub use query_ast::*;
pub use query_builders::*;
pub use response_ir::*;
pub use schema_builder::*;
pub use error::*;
pub use executor::*;
pub use schema::*;
pub use query_document::*;
pub use interpreter::*;

pub type CoreResult<T> = Result<T, CoreError>;
