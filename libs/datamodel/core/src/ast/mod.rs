//! This module contains the data structures and parsing function for the AST of a Prisma schema.
//!
//! Most of this code has been moved to the schema-ast crate so other crates in the datamodel
//! folder ar free to depend on it.
//!
//! The responsibilities of the sub modules are as following:
//! * `reformat`: Exposes a Formatter for Prisma files. This is used e.g. by the VS Code Extension.
pub mod reformat;
pub use schema_ast::{
    ast::{
        SchemaAst,
        // This is made public for tests.
        Span,
    },
    parser::parse_schema,
};

pub(crate) use schema_ast::{ast::*, renderer::Renderer};
