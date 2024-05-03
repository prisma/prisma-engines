mod helpers;
mod parse_arguments;
mod parse_attribute;
mod parse_comments;
mod parse_composite_type;
mod parse_enum;
mod parse_expression;
mod parse_field;
mod parse_model;
mod parse_schema;
mod parse_source_and_generator;
mod parse_types;
mod parse_view;

pub use parse_schema::parse_schema;

// The derive is placed here because it generates the `Rule` enum which is used in all parsing functions.
// It is more convenient if this enum is directly available here.
#[derive(pest_derive::Parser)]
#[grammar = "parser/datamodel.pest"]
#[allow(clippy::empty_docs)]
pub(crate) struct PrismaDatamodelParser;
