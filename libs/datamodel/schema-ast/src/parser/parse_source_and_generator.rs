use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_comments::*,
    parse_expression::parse_expression,
    Rule,
};
use crate::ast::*;
use diagnostics::{DatamodelError, Diagnostics};

pub(crate) fn parse_config_block(token: &Token<'_>, diagnostics: &mut Diagnostics) -> Top {
    let mut name: Option<Identifier> = None;
    let mut properties = Vec::new();
    let mut comment: Option<Comment> = None;
    let mut kw = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::key_value => properties.push(parse_key_value(&current)),
            Rule::comment_block => comment = parse_comment_block(&current),
            Rule::DATASOURCE_KEYWORD | Rule::GENERATOR_KEYWORD => kw = Some(current.as_str()),
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                format!(
                    "This line is not a valid definition within a {}.",
                    kw.unwrap_or("configuration block")
                ),
                current.as_span().into(),
            )),
            _ => parsing_catch_all(&current, "source"),
        }
    }

    match kw {
        Some("datasource") => Top::Source(SourceConfig {
            name: name.unwrap(),
            properties,
            documentation: comment,
            span: Span::from(token.as_span()),
        }),
        Some("generator") => Top::Generator(GeneratorConfig {
            name: name.unwrap(),
            properties,
            documentation: comment,
            span: Span::from(token.as_span()),
        }),
        _ => unreachable!(),
    }
}

fn parse_key_value(token: &Token<'_>) -> ConfigBlockProperty {
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
        (Some(name), Some(value)) => ConfigBlockProperty {
            name,
            value,
            span: Span::from(token.as_span()),
        },
        _ => unreachable!(
            "Encountered impossible source property declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}
