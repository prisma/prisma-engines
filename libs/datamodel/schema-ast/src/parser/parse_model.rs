use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_attribute::parse_attribute,
    parse_comments::*,
    parse_field::parse_field,
    Rule,
};
use crate::ast::*;
use diagnostics::{DatamodelError, Diagnostics};

pub(crate) fn parse_model(token: &Token<'_>, diagnostics: &mut Diagnostics) -> Model {
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = Vec::new();
    let mut fields: Vec<Field> = Vec::new();
    let mut comment: Option<Comment> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::MODEL_KEYWORD => (),
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::block_level_attribute => attributes.push(parse_attribute(&current)),
            Rule::field_declaration => match parse_field(&name.as_ref().unwrap().name, &current) {
                Ok(field) => fields.push(field),
                Err(err) => diagnostics.push_error(err),
            },
            Rule::comment_block => comment = parse_comment_block(&current),
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                "This line is not a valid field or attribute definition.".to_owned(),
                current.as_span().into(),
            )),
            _ => parsing_catch_all(&current, "model"),
        }
    }

    match name {
        Some(name) => Model {
            name,
            fields,
            attributes,
            documentation: comment,
            span: Span::from(token.as_span()),
            commented_out: false,
        },
        _ => panic!(
            "Encountered impossible model declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}
