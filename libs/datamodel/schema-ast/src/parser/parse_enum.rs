use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_attribute::parse_attribute,
    parse_comments::*,
    Diagnostics, ParserError, Rule,
};
use crate::ast::{Attribute, Comment, Enum, EnumValue, Identifier, Span};

pub fn parse_enum(token: &Token<'_>, diagnostics: &mut Diagnostics) -> Enum {
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = vec![];
    let mut values: Vec<EnumValue> = vec![];
    let mut comment: Option<Comment> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::block_level_attribute => attributes.push(parse_attribute(&current)),
            Rule::enum_value_declaration => match parse_enum_value(&name.as_ref().unwrap().name, &current) {
                Ok(enum_value) => values.push(enum_value),
                Err(err) => diagnostics.push(err),
            },
            Rule::comment_block => comment = parse_comment_block(&current),
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push(ParserError::new_validation_error(
                "This line is not an enum value definition.".to_owned(),
                current.as_span(),
            )),
            _ => parsing_catch_all(&current, "enum"),
        }
    }

    match name {
        Some(name) => Enum {
            name,
            values,
            attributes,
            documentation: comment,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible enum declaration during parsing, name is missing: {:?}",
            token.as_str()
        ),
    }
}

fn parse_enum_value(enum_name: &str, token: &Token<'_>) -> Result<EnumValue, ParserError> {
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = vec![];
    let mut comments: Vec<String> = vec![];

    // todo validate that the identifier is valid???
    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::maybe_empty_identifier => name = Some(current.to_id()),
            Rule::attribute => attributes.push(parse_attribute(&current)),
            Rule::number => {
                return Err(ParserError::new_enum_validation_error(
                    format!(
                        "The enum value `{}` is not valid. Enum values must not start with a number.",
                        current.as_str()
                    ),
                    enum_name.to_owned(),
                    token.as_span(),
                ));
            }
            Rule::doc_comment => {
                comments.push(parse_doc_comment(&current));
            }
            Rule::doc_comment_and_new_line => {
                comments.push(parse_doc_comment(&current));
            }
            _ => parsing_catch_all(&current, "enum value"),
        }
    }

    match name {
        Some(name) => Ok(EnumValue {
            name,
            attributes,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
            commented_out: false,
        }),
        _ => panic!(
            "Encountered impossible enum value declaration during parsing, name is missing: {:?}",
            token.as_str()
        ),
    }
}
