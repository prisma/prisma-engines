//! The Prisma Schema AST.
//!
//! - The `ast` module defines the AST data structure.
//! - The `parser` module is responsible from going from a source string to an AST.

#![deny(rust_2018_idioms, unsafe_code)]

pub use parser::parse_schema;
pub use reformat::reformat;

/// The AST data structure. It aims to faithfully represent the syntax of a Prisma Schema, with
/// source span information.
pub mod ast;

/// String -> AST
mod parser;

/// AST -> String
pub mod renderer;

/// String -> String
mod reformat;

/// The PSL content.
pub mod source_file;
