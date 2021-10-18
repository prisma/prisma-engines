use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_comments::parse_comment_block,
    parse_field::parse_field,
    Rule,
};
use crate::ast;
use crate::diagnostics::{DatamodelError, Diagnostics};

pub(crate) fn parse_composite_type(token: &Token<'_>, diagnostics: &mut Diagnostics) -> ast::CompositeType {
    let mut name: Option<ast::Identifier> = None;
    let mut fields: Vec<ast::Field> = vec![];
    let mut comment: Option<ast::Comment> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::TYPE_KEYWORD => (),
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::block_level_attribute => diagnostics.push_error(DatamodelError::new_validation_error(
                "Composite types cannot have block attributes.",
                ast::Span::from_pest(current.as_span()),
            )),
            Rule::field_declaration => match parse_field(&name.as_ref().unwrap().name, &current) {
                Ok(field) => fields.push(field),
                Err(err) => diagnostics.push_error(err),
            },
            Rule::comment_block => comment = parse_comment_block(&current),
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                "This line is not a valid field or attribute definition.",
                ast::Span::from_pest(current.as_span()),
            )),
            _ => parsing_catch_all(&current, "composite type"),
        }
    }

    match name {
        Some(name) => ast::CompositeType {
            name,
            fields,
            documentation: comment,
            span: ast::Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible model declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}
