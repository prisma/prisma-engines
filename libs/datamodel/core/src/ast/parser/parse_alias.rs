use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_attribute::parse_attribute,
    parse_comments::parse_comment_block,
    parse_types::parse_base_type,
    Rule,
};
use crate::ast::*;

pub fn parse_alias(token: &Token<'_>) -> Field {
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = vec![];
    let mut base_type: Option<FieldType> = None;
    let mut comment: Option<Comment> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::ALIAS_KEYWORD => {}
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::base_type => base_type = Some(parse_base_type(&current)),
            Rule::attribute => attributes.push(parse_attribute(&current)),
            Rule::comment_block => comment = parse_comment_block(&current),
            _ => parsing_catch_all(&current, "custom type"),
        }
    }

    match (name, base_type) {
        (Some(name), Some(field_type)) => Field {
            field_type,
            name,
            arity: FieldArity::Required,
            attributes,
            documentation: comment,
            span: Span::from_pest(token.as_span()),
            is_commented_out: false,
        },
        _ => panic!(
            "Encountered impossible custom type declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}
