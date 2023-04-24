#![deny(rust_2018_idioms, unsafe_code)]

pub mod constants;

mod build;
mod db;
mod enum_type;
mod identifier_type;
mod input_types;
mod output_types;
mod query_schema;
mod utils;

pub use self::build::{build, build_with_features, compound_id_field_name, compound_index_field_name};
pub use db::*;
pub use enum_type::*;
pub use identifier_type::*;
pub use input_types::*;
pub use output_types::*;
pub use query_schema::*;
pub use utils::*;

use std::sync::Arc;

pub type QuerySchemaRef = Arc<QuerySchema>;
