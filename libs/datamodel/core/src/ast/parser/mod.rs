mod helpers;
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

pub use parse_schema::parse_schema;

// The derive is placed here because it generates the `Rule` enum which is used in all parsing functions.
// It is more convenient if this enum is directly available here.
#[derive(pest_derive::Parser)]
#[grammar = "ast/parser/datamodel.pest"]
pub struct PrismaDatamodelParser;
