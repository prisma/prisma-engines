use super::{
    helpers::{parsing_catch_all, Pair},
    parse_attribute::parse_attribute,
    parse_comments::*,
    Rule,
};
use crate::ast::{Attribute, Comment, Enum, EnumValue, Identifier};
use diagnostics::{DatamodelError, Diagnostics, Span};

pub fn parse_enum(pair: Pair<'_>, doc_comment: Option<Pair<'_>>, diagnostics: &mut Diagnostics) -> Enum {
    let comment: Option<Comment> = doc_comment.and_then(parse_comment_block);
    let pair_span = pair.as_span();
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = vec![];
    let mut values: Vec<EnumValue> = vec![];
    let mut pending_value_comment = None;
    let mut pairs = pair.into_inner().peekable();

    while let Some(current) = pairs.next() {
        match current.as_rule() {
            Rule::BLOCK_OPEN | Rule::BLOCK_CLOSE | Rule::ENUM_KEYWORD => {}
            Rule::identifier => name = Some(current.into()),
            Rule::block_attribute => attributes.push(parse_attribute(current, diagnostics)),
            Rule::enum_value_declaration => {
                match parse_enum_value(current, pending_value_comment.take(), diagnostics) {
                    Ok(enum_value) => values.push(enum_value),
                    Err(err) => diagnostics.push_error(err),
                }
            }
            Rule::comment_block => {
                if let Some(Rule::enum_value_declaration) = pairs.peek().map(|t| t.as_rule()) {
                    pending_value_comment = Some(current);
                }
            }
            Rule::BLOCK_LEVEL_CATCH_ALL => diagnostics.push_error(DatamodelError::new_validation_error(
                "This line is not an enum value definition.",
                current.as_span().into(),
            )),
            _ => parsing_catch_all(&current, "enum"),
        }
    }

    match name {
        Some(name) => Enum {
            name,
            values,
            attributes,
            documentation: comment,
            span: Span::from(pair_span),
        },
        _ => panic!("Encountered impossible enum declaration during parsing, name is missing.",),
    }
}

fn parse_enum_value(
    pair: Pair<'_>,
    doc_comment: Option<Pair<'_>>,
    diagnostics: &mut Diagnostics,
) -> Result<EnumValue, DatamodelError> {
    let (pair_str, pair_span) = (pair.as_str(), pair.as_span());
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = vec![];
    let mut comment: Option<Comment> = doc_comment.and_then(parse_comment_block);

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::identifier => name = Some(current.into()),
            Rule::field_attribute => attributes.push(parse_attribute(current, diagnostics)),
            Rule::trailing_comment => {
                comment = match (comment, parse_trailing_comment(current)) {
                    (None, a) | (a, None) => a,
                    (Some(a), Some(b)) => Some(Comment {
                        text: [a.text, b.text].join("\n"),
                    }),
                };
            }
            Rule::comment_block => {
                parse_comment_block(current);
            }
            _ => parsing_catch_all(&current, "enum value"),
        }
    }

    match name {
        Some(name) => Ok(EnumValue {
            name,
            attributes,
            documentation: comment,
            span: Span::from(pair_span),
        }),
        _ => panic!("Encountered impossible enum value declaration during parsing, name is missing: {pair_str:?}",),
    }
}
