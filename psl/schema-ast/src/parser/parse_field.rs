use super::{
    helpers::{parsing_catch_all, Pair},
    parse_attribute::parse_attribute,
    parse_comments::*,
    parse_types::parse_field_type,
    Rule,
};
use crate::ast::*;
use diagnostics::{DatamodelError, Diagnostics};

pub(crate) fn parse_field(
    model_name: &str,
    pair: Pair<'_>,
    block_comment: Option<Pair<'_>>,
    diagnostics: &mut Diagnostics,
) -> Result<Field, DatamodelError> {
    let pair_span = pair.as_span();
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = Vec::new();
    let mut field_type: Option<(FieldArity, FieldType)> = None;
    let mut comment: Option<Comment> = block_comment.and_then(parse_comment_block);

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::identifier => name = Some(current.into()),
            Rule::field_type => field_type = Some(parse_field_type(current, diagnostics)?),
            Rule::LEGACY_COLON => {
                return Err(DatamodelError::new_legacy_parser_error(
                    "Field declarations don't require a `:`.",
                    current.as_span().into(),
                ))
            }
            Rule::field_attribute => attributes.push(parse_attribute(current, diagnostics)),
            Rule::trailing_comment => {
                comment = match (comment, parse_trailing_comment(current)) {
                    (c, None) | (None, c) => c,
                    (Some(existing), Some(new)) => Some(Comment {
                        text: [existing.text, new.text].join("\n"),
                    }),
                };
            }
            _ => parsing_catch_all(&current, "field"),
        }
    }

    match (name, field_type) {
        (Some(name), Some((arity, field_type))) => Ok(Field {
            field_type,
            name,
            arity,
            attributes,
            documentation: comment,
            span: Span::from(pair_span),
        }),
        _ => Err(DatamodelError::new_model_validation_error(
            "This field declaration is invalid. It is either missing a name or a type.",
            model_name,
            pair_span.into(),
        )),
    }
}
