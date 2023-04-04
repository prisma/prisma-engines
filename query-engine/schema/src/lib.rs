#![deny(rust_2018_idioms, unsafe_code)]

mod db;
mod enum_type;
mod identifier_type;
mod input_types;
mod output_types;
mod query_schema;
mod utils;

pub use db::*;
pub use enum_type::*;
pub use identifier_type::*;
pub use input_types::*;
pub use output_types::*;
pub use query_schema::*;
pub use utils::*;

use std::sync::Arc;

pub type QuerySchemaRef = Arc<QuerySchema>;
pub type OutputTypeRef = Arc<OutputType>;

#[derive(Debug, PartialEq)]
pub struct Deprecation {
    pub since_version: String,
    pub planned_removal_version: Option<String>,
    pub reason: String,
}
