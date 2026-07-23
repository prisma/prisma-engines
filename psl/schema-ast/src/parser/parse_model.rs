use super::{
    Rule,
    helpers::{Pair, parsing_catch_all},
    parse_attribute::parse_attribute,
    parse_comments::*,
    parse_field::parse_field,
};
use crate::ast::{self, *};
use diagnostics::{DatamodelError, Diagnostics, FileId};

pub(crate) fn parse_model(
    pair: Pair<'_>,
    doc_comment: Option<Pair<'_>>,
    diagnostics: &mut Diagnostics,
    file_id: FileId,
) -> Model {
    let pair_span = pair.as_span();
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = Vec::new();
    let mut fields: Vec<Field> = Vec::new();

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::MODEL_KEYWORD | Rule::BLOCK_OPEN | Rule::BLOCK_CLOSE => {}
            Rule::identifier => name = Some(ast::Identifier::new(current, file_id)),
            Rule::model_contents => {
                let mut pending_field_comment: Option<Pair<'_>> = None;

                for item in current.into_inner() {
                    match item.as_rule() {
                        Rule::block_attribute => attributes.push(parse_attribute(item, diagnostics, file_id)),
                        Rule::field_declaration => match parse_field(
                            &name.as_ref().unwrap().name,
                            "model",
                            item,
                            pending_field_comment.take(),
                            diagnostics,
                            file_id,
                        ) {
                            Ok(field) => fields.push(field),
                            Err(err) => diagnostics.push_error(err),
                        },
                        Rule::comment_block => pending_field_comment = Some(item),
                        Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                            "This line is not a valid field or attribute definition.",
                            (file_id, item.as_span()).into(),
                        )),
                        _ => parsing_catch_all(&item, "model"),
                    }
                }
            }
            _ => parsing_catch_all(&current, "model"),
        }
    }

    match name {
        Some(name) => Model {
            name,
            fields,
            attributes,
            documentation: doc_comment.and_then(parse_comment_block),
            is_view: false,
            span: Span::from((file_id, pair_span)),
        },
        _ => panic!("Encountered impossible model declaration during parsing",),
    }
}
