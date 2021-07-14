#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects

//! The top-level library crate for the migration engine.

mod api;
mod core_error;

pub mod commands;

pub use core_error::*;

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

use datamodel::{Configuration, Datamodel};

fn parse_schema(schema: &str) -> CoreResult<(Configuration, Datamodel)> {
    datamodel::parse_schema(schema).map_err(CoreError::new_schema_parser_error)
}
