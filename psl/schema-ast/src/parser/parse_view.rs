use super::{
    helpers::{parsing_catch_all, Pair},
    parse_attribute::parse_attribute,
    parse_comments::parse_comment_block,
    parse_field::parse_field,
    Rule,
};
use crate::ast::{self, Attribute};
use diagnostics::{DatamodelError, Diagnostics};

pub(crate) fn parse_view(pair: Pair<'_>, doc_comment: Option<Pair<'_>>, diagnostics: &mut Diagnostics) -> ast::Model {
    let pair_span = pair.as_span();
    let mut name: Option<ast::Identifier> = None;
    let mut fields: Vec<ast::Field> = vec![];
    let mut attributes: Vec<Attribute> = Vec::new();

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::VIEW_KEYWORD | Rule::BLOCK_OPEN | Rule::BLOCK_CLOSE => (),
            Rule::identifier => name = Some(current.into()),
            Rule::model_contents => {
                let mut pending_field_comment: Option<Pair<'_>> = None;

                for item in current.into_inner() {
                    match item.as_rule() {
                        Rule::block_attribute => attributes.push(parse_attribute(item, diagnostics)),
                        Rule::field_declaration => match parse_field(
                            &name.as_ref().unwrap().name,
                            "view",
                            item,
                            pending_field_comment.take(),
                            diagnostics,
                        ) {
                            Ok(field) => fields.push(field),
                            Err(err) => diagnostics.push_error(err),
                        },
                        Rule::comment_block => pending_field_comment = Some(item),
                        Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                            "This line is not a valid field or attribute definition.",
                            item.as_span().into(),
                        )),
                        _ => parsing_catch_all(&item, "view"),
                    }
                }
            }
            _ => parsing_catch_all(&current, "view"),
        }
    }

    match name {
        Some(name) => ast::Model {
            name,
            fields,
            attributes,
            documentation: doc_comment.and_then(parse_comment_block),
            is_view: true,
            span: ast::Span::from(pair_span),
        },
        _ => panic!("Encountered impossible model declaration during parsing",),
    }
}
