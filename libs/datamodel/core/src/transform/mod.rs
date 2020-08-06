//! This module is responsible for converting between the AST and DML data structures.
//!
//! The responsibilities of the sub modules are:
//! * `ast_to_dml` contains functionality to convert an AST into a DML data structure. This can error as validation is performed during this process.
//! * `dml_to_ast` contains functionality to convert a DML structure back to an AST. This is used for rendering and can not fail.
//! * `directives` contains the logic for validating and applying the effects of directives like e.g. `@default`.
//! * `helpers` contains helpers to simplify the validation of arguments and values in the AST during validation.
mod directives;
mod helpers;

pub mod ast_to_dml;
pub mod dml_to_ast;
