use super::{
    helpers::{parsing_catch_all, Pair},
    parse_attribute::parse_attribute,
    parse_comments::parse_comment_block,
    parse_field::parse_field,
    Rule,
};
use crate::ast;
use diagnostics::{DatamodelError, Diagnostics, Span};

pub(crate) fn parse_composite_type(
    pair: Pair<'_>,
    doc_comment: Option<Pair<'_>>,
    diagnostics: &mut Diagnostics,
) -> ast::CompositeType {
    let pair_span = pair.as_span();
    let mut name: Option<ast::Identifier> = None;
    let mut fields: Vec<ast::Field> = vec![];
    let mut pairs = pair.into_inner();
    let mut inner_span: Option<Span> = None;

    while let Some(current) = pairs.next() {
        match current.as_rule() {
            Rule::BLOCK_OPEN | Rule::BLOCK_CLOSE => {}
            Rule::TYPE_KEYWORD => (),
            Rule::identifier => name = Some(current.into()),
            Rule::model_contents => {
                let mut pending_field_comment: Option<Pair<'_>> = None;
                inner_span = Some(current.as_span().into());

                for item in current.into_inner() {
                    let current_span = item.as_span();

                    match item.as_rule() {
                        Rule::block_attribute => {
                            let attr = parse_attribute(item, diagnostics);

                            let err = match attr.name.name.as_str() {
                                "map" => {
                                    DatamodelError::new_validation_error(
                                        "The name of a composite type is not persisted in the database, therefore it does not need a mapped database name.",
                                        current_span.into(),
                                    )
                                }
                                "unique" => {
                                    DatamodelError::new_validation_error(
                                        "A unique constraint should be defined in the model containing the embed.",
                                        current_span.into(),
                                    )
                                }
                                "index" => {
                                    DatamodelError::new_validation_error(
                                        "An index should be defined in the model containing the embed.",
                                        current_span.into(),
                                    )
                                }
                                "fulltext" => {
                                    DatamodelError::new_validation_error(
                                        "A fulltext index should be defined in the model containing the embed.",
                                        current_span.into(),
                                    )
                                }
                                "id" => {
                                    DatamodelError::new_validation_error(
                                        "A composite type cannot define an id.",
                                        current_span.into(),
                                    )
                                }
                                _ => {
                                    DatamodelError::new_validation_error(
                                        "A composite type cannot have block-level attributes.",
                                        current_span.into(),
                                    )
                                }
                            };

                            diagnostics.push_error(err);
                        }
                        Rule::field_declaration => match parse_field(
                            &name.as_ref().unwrap().name,
                            "type",
                            item,
                            pending_field_comment.take(),
                            diagnostics,
                        ) {
                            Ok(field) => {
                                for attr in field.attributes.iter() {
                                    let error = match attr.name.name.as_str() {
                                        "relation" | "unique" | "id" => {
                                            let name = attr.name.name.as_str();

                                            let msg = format!(
                                                "Defining `@{name}` attribute for a field in a composite type is not allowed."
                                            );

                                            DatamodelError::new_validation_error(&msg, current_span.into())
                                        }
                                        _ => continue,
                                    };

                                    diagnostics.push_error(error);
                                }

                                fields.push(field)
                            }
                            Err(err) => diagnostics.push_error(err),
                        },
                        Rule::comment_block => {
                            if let Some(Rule::field_declaration) = pairs.peek().map(|p| p.as_rule()) {
                                pending_field_comment = Some(item);
                            }
                        }
                        Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                            "This line is not a valid field or attribute definition.",
                            item.as_span().into(),
                        )),
                        _ => parsing_catch_all(&item, "composite type"),
                    }
                }
            }
            _ => parsing_catch_all(&current, "composite type"),
        }
    }

    match name {
        Some(name) => ast::CompositeType {
            name,
            fields,
            documentation: doc_comment.and_then(parse_comment_block),
            span: ast::Span::from(pair_span),
            inner_span: inner_span.unwrap(),
        },
        _ => panic!("Encountered impossible model declaration during parsing",),
    }
}
