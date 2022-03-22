use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_attribute::parse_attribute,
    parse_comments::parse_comment_block,
    parse_field::parse_field,
    Rule,
};
use crate::ast;
use diagnostics::{DatamodelError, Diagnostics};

pub(crate) fn parse_composite_type(token: &Token<'_>, diagnostics: &mut Diagnostics) -> ast::CompositeType {
    let mut name: Option<ast::Identifier> = None;
    let mut fields: Vec<ast::Field> = vec![];
    let mut comment: Option<ast::Comment> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::TYPE_KEYWORD => (),
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::block_level_attribute => {
                let attr = parse_attribute(&current);

                let err = match attr.name.name.as_str() {
                    "map" => {
                        DatamodelError::new_validation_error(
                            "The name of a composite type is not persisted in the database, therefore it does not need a mapped database name."
                                .to_owned(),
                            current.as_span().into(),
                        )
                    }
                    "unique" => {
                        DatamodelError::new_validation_error(
                            "A unique constraint should be defined in the model containing the embed."
                                .to_owned(),
                            current.as_span().into(),
                        )
                    }
                    "index" => {
                        DatamodelError::new_validation_error(
                            "An index should be defined in the model containing the embed."
                                .to_owned(),
                            current.as_span().into(),
                        )
                    }
                    "fulltext" => {
                        DatamodelError::new_validation_error(
                            "A fulltext index should be defined in the model containing the embed."
                                .to_owned(),
                            current.as_span().into(),
                        )
                    }
                    "id" => {
                        DatamodelError::new_validation_error(
                            "A composite type cannot define an id."
                                .to_owned(),
                            current.as_span().into(),
                        )
                    }
                    _ => {
                        DatamodelError::new_validation_error(
                            "A composite type cannot have block-level attributes."
                                .to_owned(),
                            current.as_span().into(),
                        )
                    }
                };

                diagnostics.push_error(err);
            }
            Rule::field_declaration => match parse_field(&name.as_ref().unwrap().name, &current) {
                Ok(field) => {
                    for attr in field.attributes.iter() {
                        let error = match attr.name.name.as_str() {
                            "relation" | "unique" | "id" => {
                                let name = attr.name.name.as_str();

                                let msg = format!(
                                    "Defining `@{name}` attribute for a field in a composite type is not allowed."
                                );

                                DatamodelError::new_validation_error(msg, current.as_span().into())
                            }
                            _ => continue,
                        };

                        diagnostics.push_error(error);
                    }

                    fields.push(field)
                }
                Err(err) => diagnostics.push_error(err),
            },
            Rule::comment_block => comment = parse_comment_block(&current),
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                "This line is not a valid field or attribute definition.".to_owned(),
                current.as_span().into(),
            )),
            _ => parsing_catch_all(&current, "composite type"),
        }
    }

    match name {
        Some(name) => ast::CompositeType {
            name,
            fields,
            documentation: comment,
            span: ast::Span::from(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible model declaration during parsing: {:?}",
            token.as_str()
        ),
    }
}
