use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_comments::*,
    parse_field::parse_field,
    Rule,
};
use crate::ast::*;
use crate::diagnostics::{DatamodelError, Diagnostics};

pub fn parse_type_definition(token: &Token<'_>, diagnostics: &mut Diagnostics) -> TypeDefinition {
    let mut name: Option<Identifier> = None;
    let mut fields: Vec<Field> = vec![];
    let mut comment: Option<Comment> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::TYPE_KEYWORD => diagnostics.push_error(DatamodelError::new_legacy_parser_error(
                "Type declarations have to be indicated with the `type` keyword.",
                Span::from_pest(current.as_span()),
            )),
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::field_declaration => match parse_field(&name.as_ref().unwrap().name, &current) {
                Ok(field) => fields.push(field),
                Err(err) => diagnostics.push_error(err),
            },
            Rule::comment_block => comment = parse_comment_block(&current),
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                "This line is not a valid field definition.",
                Span::from_pest(current.as_span()),
            )),
            _ => parsing_catch_all(&current, "model"),
        }
    }

    match name {
        Some(name) => TypeDefinition {
            name,
            fields,
            documentation: comment,
            span: Span::from_pest(token.as_span()),
            commented_out: false,
        },
        _ => panic!(
            "Encountered impossible type declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}
