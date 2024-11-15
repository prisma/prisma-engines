#![deny(rust_2018_idioms, unsafe_code)]

pub mod constants;

mod build;
mod enum_type;
mod identifier_type;
mod input_types;
mod output_types;
mod query_schema;
mod utils;

pub use self::{
    build::{build, build_with_features, compound_id_field_name, compound_index_field_name, itx_isolation_levels},
    enum_type::{DatabaseEnumType, EnumType},
    input_types::{InputField, InputObjectType, InputType, ObjectTag},
    output_types::{InnerOutputType, ObjectType, OutputField, OutputType},
    query_schema::{Identifier, QueryInfo, QuerySchema, QueryTag, ScalarType},
};

use self::{identifier_type::IdentifierType, input_types::InputObjectTypeConstraints, utils::*};
use std::sync::Arc;

pub type QuerySchemaRef = Arc<QuerySchema>;
