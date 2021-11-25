use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_comments::*,
    parse_expression::parse_expression,
    Diagnostics, ParserError, Rule,
};
use crate::ast::*;

pub(crate) fn parse_source(token: &Token<'_>, diagnostics: &mut Diagnostics) -> SourceConfig {
    let mut name: Option<Identifier> = None;
    let mut properties: Vec<Argument> = vec![];
    let mut comment: Option<Comment> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::key_value => properties.push(parse_key_value(&current)),
            Rule::comment_block => comment = parse_comment_block(&current),
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push(ParserError::new_validation_error(
                "This line is not a valid definition within a datasource.".to_owned(),
                current.as_span(),
            )),
            _ => parsing_catch_all(&current, "source"),
        }
    }

    match name {
        Some(name) => SourceConfig {
            name,
            properties,
            documentation: comment,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible source declaration during parsing, name is missing: {:?}",
            token.as_str()
        ),
    }
}

pub fn parse_generator(token: &Token<'_>, diagnostics: &mut Diagnostics) -> GeneratorConfig {
    let mut name: Option<Identifier> = None;
    let mut properties: Vec<Argument> = vec![];
    let mut comments: Vec<String> = Vec::new();

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::key_value => properties.push(parse_key_value(&current)),
            Rule::doc_comment => comments.push(parse_doc_comment(&current)),
            Rule::doc_comment_and_new_line => comments.push(parse_doc_comment(&current)),
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push(ParserError::new_validation_error(
                "This line is not a valid definition within a generator.".to_owned(),
                current.as_span(),
            )),
            _ => parsing_catch_all(&current, "generator"),
        }
    }

    match name {
        Some(name) => GeneratorConfig {
            name,
            properties,
            documentation: doc_comments_to_string(&comments),
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible generator declaration during parsing, name is missing: {:?}",
            token.as_str()
        ),
    }
}

fn parse_key_value(token: &Token<'_>) -> Argument {
    let mut name: Option<Identifier> = None;
    let mut value: Option<Expression> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::expression => value = Some(parse_expression(&current)),
            _ => unreachable!(
                "Encountered impossible source property declaration during parsing: {:?}",
                current.tokens()
            ),
        }
    }

    match (name, value) {
        (Some(name), Some(value)) => Argument {
            name,
            value,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible source property declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}
