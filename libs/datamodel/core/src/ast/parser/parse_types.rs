use super::{helpers::ToIdentifier, parse_comments::parse_comment_block, parse_directive::parse_directive, Rule};
use crate::ast::*;
use crate::error::DatamodelError;

pub fn parse_type_alias(token: &pest::iterators::Pair<'_, Rule>) -> Field {
    let mut name: Option<Identifier> = None;
    let mut directives: Vec<Directive> = vec![];
    let mut base_type: Option<(String, Span)> = None;
    let mut comment: Option<Comment> = None;

    match_children! { token, current,
        Rule::TYPE_KEYWORD => { },
        Rule::non_empty_identifier => name = Some(current.to_id()),
        Rule::base_type => {
            base_type = Some((parse_base_type(&current), Span::from_pest(current.as_span())))
        },
        Rule::directive => directives.push(parse_directive(&current)),
        Rule::comment_block => {
            comment = Some(parse_comment_block(&current))
        },
        _ => unreachable!("Encountered impossible custom type during parsing: {:?}", current.tokens())
    }

    match (name, base_type) {
        (Some(name), Some((field_type, field_type_span))) => Field {
            field_type: Identifier {
                name: field_type,
                span: field_type_span,
            },
            name,
            arity: FieldArity::Required,
            directives,
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

pub fn parse_field_type(token: &pest::iterators::Pair<'_, Rule>) -> Result<(FieldArity, String), DatamodelError> {
    match_first! { token, current,
        Rule::optional_type => Ok((FieldArity::Optional, parse_base_type(&current))),
        Rule::base_type =>  Ok((FieldArity::Required, parse_base_type(&current))),
        Rule::list_type =>  Ok((FieldArity::List, parse_base_type(&current))),
        Rule::legacy_required_type => Err(DatamodelError::new_legacy_parser_error(
            "Fields are required by default, `!` is no longer required.",
            Span::from_pest(current.as_span())
        )),
        Rule::legacy_list_type => Err(DatamodelError::new_legacy_parser_error(
            "To specify a list, please use `Type[]` instead of `[Type]`.",
            Span::from_pest(current.as_span())
        )),
        Rule::unsupported_optional_list_type => Err(DatamodelError::new_legacy_parser_error(
            "Optional lists are not supported. Use either `Type[]` or `Type?`.",
            Span::from_pest(current.as_span())
        )),
        _ => unreachable!("Encountered impossible field during parsing: {:?}", current.tokens())
    }
}

fn parse_base_type(token: &pest::iterators::Pair<'_, Rule>) -> String {
    match_first! { token, current,
        Rule::non_empty_identifier => current.as_str().to_string(),
        _ => unreachable!("Encountered impossible type during parsing: {:?}", current.tokens())
    }
}
