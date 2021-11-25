use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_attribute::parse_attribute,
    parse_comments::parse_comment_block,
    ParserError, Rule,
};
use crate::{ast::*, parser::parse_expression::parse_expression};

pub fn parse_type_alias(token: &Token<'_>) -> Field {
    let mut name: Option<Identifier> = None;
    let mut attributes: Vec<Attribute> = vec![];
    let mut base_type: Option<FieldType> = None;
    let mut comment: Option<Comment> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::TYPE_KEYWORD => {}
            Rule::non_empty_identifier => name = Some(current.to_id()),
            Rule::base_type => base_type = Some(parse_base_type(&current)),
            Rule::attribute => attributes.push(parse_attribute(&current)),
            Rule::comment_block => comment = parse_comment_block(&current),
            _ => parsing_catch_all(&current, "custom type"),
        }
    }

    match (name, base_type) {
        (Some(name), Some(field_type)) => Field {
            field_type,
            name,
            arity: FieldArity::Required,
            attributes,
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

pub fn parse_field_type(token: &Token<'_>) -> Result<(FieldArity, FieldType), ParserError> {
    let current = token.first_relevant_child();
    match current.as_rule() {
        Rule::optional_type => Ok((FieldArity::Optional, parse_base_type(&current.first_relevant_child()))),
        Rule::base_type => Ok((FieldArity::Required, parse_base_type(&current))),
        Rule::list_type => Ok((FieldArity::List, parse_base_type(&current.first_relevant_child()))),
        Rule::legacy_required_type => Err(ParserError::new_legacy_parser_error(
            "Fields are required by default, `!` is no longer required.",
            current.as_span(),
        )),
        Rule::legacy_list_type => Err(ParserError::new_legacy_parser_error(
            "To specify a list, please use `Type[]` instead of `[Type]`.",
            current.as_span(),
        )),
        Rule::unsupported_optional_list_type => Err(ParserError::new_legacy_parser_error(
            "Optional lists are not supported. Use either `Type[]` or `Type?`.",
            current.as_span(),
        )),
        _ => unreachable!("Encountered impossible field during parsing: {:?}", current.tokens()),
    }
}

fn parse_base_type(token: &Token<'_>) -> FieldType {
    let current = token.first_relevant_child();
    match current.as_rule() {
        Rule::non_empty_identifier => FieldType::Supported(Identifier {
            name: current.as_str().to_string(),
            span: Span::from_pest(current.as_span()),
        }),
        Rule::unsupported_type => match parse_expression(&current) {
            Expression::StringValue(lit, span) => FieldType::Unsupported(lit, span),
            _ => unreachable!("Encountered impossible type during parsing: {:?}", current.tokens()),
        },
        _ => unreachable!("Encountered impossible type during parsing: {:?}", current.tokens()),
    }
}
