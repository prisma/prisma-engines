#![deny(warnings)]
#![macro_use]
extern crate failure_derive;

pub mod error;
pub mod filter;
pub mod result_ast;

mod compare;
mod interface;
mod query_arguments;
mod write_args;

pub use compare::*;
pub use filter::*;
pub use interface::*;
pub use query_arguments::*;
pub use result_ast::*;
pub use write_args::*;

pub type Result<T> = std::result::Result<T, error::ConnectorError>;
