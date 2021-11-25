use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_attribute::parse_attribute,
    parse_comments::*,
    parse_types::parse_field_type,
    ParserError, Rule,
};
use crate::ast::*;

pub fn parse_field(model_name: &str, token: &Token<'_>) -> Result<Field, ParserError> {
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = Vec::new();
    let mut field_type: Option<(FieldArity, FieldType)> = None;
    let mut comments: Vec<String> = Vec::new();

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::field_type => field_type = Some(parse_field_type(&current)?),
            Rule::LEGACY_COLON => {
                return Err(ParserError::new_legacy_parser_error(
                    "Field declarations don't require a `:`.",
                    current.as_span(),
                ))
            }
            Rule::attribute => attributes.push(parse_attribute(&current)),
            Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&current)),
            Rule::doc_comment => comments.push(parse_doc_comment(&current)),
            _ => parsing_catch_all(&current, "field"),
        }
    }

    match (name, field_type) {
        (Some(name), Some((arity, field_type))) => Ok(Field {
            field_type,
            name,
            arity,
            attributes,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
            is_commented_out: false,
        }),
        _ => Err(ParserError::new_model_validation_error(
            "This field declaration is invalid. It is either missing a name or a type.".to_owned(),
            model_name.to_owned(),
            token.as_span(),
        )),
    }
}
