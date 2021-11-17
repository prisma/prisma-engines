//! The Prisma Schema AST.

#![deny(rust_2018_idioms, unsafe_code)]

/// The AST data structure. It aims to faithfully represent the syntax of a Prisma Schema, with
/// source span information.
pub mod ast;
