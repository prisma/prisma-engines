//! The Prisma Schema AST.
//!
//! - The `ast` module defines the AST data structure.
//! - The `parser` module is responsible from going from a source string to an AST.
//! - The `renderer` module is responsible for rendering an AST to a string.

#![deny(rust_2018_idioms, unsafe_code)]

/// The AST data structure. It aims to faithfully represent the syntax of a Prisma Schema, with
/// source span information.
pub mod ast;

/// String -> AST
pub mod parser;

/// AST -> String
pub mod renderer;
