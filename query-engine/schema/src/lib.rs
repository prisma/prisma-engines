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

pub use self::{
    build::{build, build_with_features, compound_id_field_name, compound_index_field_name},
    db::{InputObjectTypeId, OutputFieldId, OutputObjectTypeId},
    enum_type::{DatabaseEnumType, EnumType},
    input_types::{InputField, InputObjectType, InputType, ObjectTag},
    output_types::{ObjectType, OutputField, OutputType},
    query_schema::{ConnectorContext, Identifier, QueryInfo, QuerySchema, QueryTag, ScalarType},
    utils::{capitalize, scalar_filter_name},
};

use self::{
    db::{EnumTypeId, QuerySchemaDatabase},
    identifier_type::IdentifierType,
    input_types::InputObjectTypeConstraints,
};
use std::sync::Arc;

pub type QuerySchemaRef = Arc<QuerySchema>;
