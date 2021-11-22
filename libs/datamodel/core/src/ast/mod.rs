//! This module contains the data structures and parsing function for the AST of a Prisma schema.
//! The responsibilities of the sub modules are as following:
//! * `parser`: Exposes the parse function that turns a String input into an AST.
//! * `reformat`: Exposes a Formatter for Prisma files. This is used e.g. by the VS Code Extension.
//! * `renderer`: Turns an AST into a Prisma Schema String.
mod helper;
mod parser;
mod renderer;

pub mod reformat;
pub use schema_ast::ast::{
    SchemaAst,
    // This is made public for tests.
    Span,
};

pub(crate) use parser::parse_schema;
pub(crate) use renderer::Renderer;
pub(crate) use schema_ast::ast::*;
