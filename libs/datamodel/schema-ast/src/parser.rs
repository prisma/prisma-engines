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

use crate::ast::Span;

pub use parse_schema::parse_schema;

// The derive is placed here because it generates the `Rule` enum which is used in all parsing functions.
// It is more convenient if this enum is directly available here.
#[derive(pest_derive::Parser)]
#[grammar = "parser/datamodel.pest"]
pub struct PrismaDatamodelParser;

pub struct Diagnostics(Vec<ParserError>);

impl Diagnostics {
    fn new() -> Self {
        Diagnostics(Vec::new())
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn push(&mut self, diagnostic: ParserError) {
        self.0.push(diagnostic)
    }
}

impl<'src> IntoIterator for Diagnostics {
    type Item = ParserError;

    type IntoIter = std::vec::IntoIter<ParserError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub enum ParserError {
    ParserError(Vec<&'static str>, Span),
    ValidationError(String, Span),
    LegacyParserError(&'static str, Span),
    EnumValidationError(String, String, Span),
    ModelValidationError(String, String, Span),
}

impl<'src> ParserError {
    fn new_legacy_parser_error(message: &'static str, location: pest::Span<'src>) -> Self {
        Self::LegacyParserError(message, location.into())
    }

    fn new_parser_error(expected: Vec<&'static str>, location: pest::Span<'src>) -> Self {
        Self::ParserError(expected, location.into())
    }

    fn new_validation_error(message: String, location: pest::Span<'src>) -> Self {
        Self::ValidationError(message, location.into())
    }

    fn new_enum_validation_error(message: String, enum_name: String, span: pest::Span<'src>) -> Self {
        Self::EnumValidationError(message, enum_name, span.into())
    }

    fn new_model_validation_error(message: String, model_name: String, span: pest::Span<'src>) -> Self {
        Self::ModelValidationError(message, model_name, span.into())
    }
}
