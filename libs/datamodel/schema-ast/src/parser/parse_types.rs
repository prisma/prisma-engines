use super::{
    helpers::{Token, TokenExtensions},
    Rule,
};
use crate::{ast::*, parser::parse_expression::parse_expression};
use diagnostics::DatamodelError;

pub fn parse_field_type(token: &Token<'_>) -> Result<(FieldArity, FieldType), DatamodelError> {
    let current = token.first_relevant_child();
    match current.as_rule() {
        Rule::optional_type => Ok((FieldArity::Optional, parse_base_type(&current.first_relevant_child()))),
        Rule::base_type => Ok((FieldArity::Required, parse_base_type(&current))),
        Rule::list_type => Ok((FieldArity::List, parse_base_type(&current.first_relevant_child()))),
        Rule::legacy_required_type => Err(DatamodelError::new_legacy_parser_error(
            "Fields are required by default, `!` is no longer required.",
            current.as_span().into(),
        )),
        Rule::legacy_list_type => Err(DatamodelError::new_legacy_parser_error(
            "To specify a list, please use `Type[]` instead of `[Type]`.",
            current.as_span().into(),
        )),
        Rule::unsupported_optional_list_type => Err(DatamodelError::new_legacy_parser_error(
            "Optional lists are not supported. Use either `Type[]` or `Type?`.",
            current.as_span().into(),
        )),
        _ => unreachable!("Encountered impossible field during parsing: {:?}", current.tokens()),
    }
}

fn parse_base_type(token: &Token<'_>) -> FieldType {
    let current = token.first_relevant_child();
    match current.as_rule() {
        Rule::non_empty_identifier => FieldType::Supported(Identifier {
            name: current.as_str().to_string(),
            span: Span::from(current.as_span()),
        }),
        Rule::unsupported_type => match parse_expression(&current) {
            Expression::StringValue(lit, span) => FieldType::Unsupported(lit, span),
            _ => unreachable!("Encountered impossible type during parsing: {:?}", current.tokens()),
        },
        _ => unreachable!("Encountered impossible type during parsing: {:?}", current.tokens()),
    }
}
