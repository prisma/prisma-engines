use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_expression::parse_arg_value,
    Rule,
};
use crate::ast::*;

pub fn parse_attribute(token: &Token<'_>) -> Attribute {
    let mut name: Option<Identifier> = None;
    let mut arguments: Vec<Argument> = Vec::new();
    let mut empty_arguments = Vec::new();

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::attribute => return parse_attribute(&current),
            Rule::attribute_name => name = Some(current.to_id()),
            Rule::attribute_arguments => parse_attribute_args(&current, &mut arguments, &mut empty_arguments),
            _ => parsing_catch_all(&current, "attribute"),
        }
    }

    match name {
        Some(name) => Attribute {
            name,
            arguments,
            empty_arguments,
            span: Span::from(token.as_span()),
        },
        _ => panic!("Encountered impossible type during parsing: {:?}", token.as_str()),
    }
}

pub(crate) fn parse_attribute_args(
    token: &Token<'_>,
    arguments: &mut Vec<Argument>,
    empty_arguments: &mut Vec<EmptyArgument>,
) {
    debug_assert_eq!(token.as_rule(), Rule::attribute_arguments);
    for current in token.relevant_children() {
        match current.as_rule() {
            // This is a named arg.
            Rule::named_argument => arguments.push(parse_attribute_arg(&current)),
            // This is an unnamed arg.
            Rule::argument_value => arguments.push(Argument {
                name: Identifier::new(""),
                value: parse_arg_value(&current),
                span: Span::from(current.as_span()),
            }),
            // This is an argument without a value.
            // It is not valid, but we parse it for autocompletion.
            Rule::empty_argument => {
                let name = current
                    .into_inner()
                    .find(|tok| tok.as_rule() == Rule::argument_name)
                    .unwrap();
                empty_arguments.push(EmptyArgument { name: name.to_id() })
            }
            _ => parsing_catch_all(&current, "attribute arguments"),
        }
    }
}

pub(crate) fn parse_attribute_arg(token: &Token<'_>) -> Argument {
    let mut name: Option<Identifier> = None;
    let mut argument: Option<Expression> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::argument_name => name = Some(current.to_id()),
            Rule::argument_value => argument = Some(parse_arg_value(&current)),
            _ => parsing_catch_all(&current, "attribute argument"),
        }
    }

    match (name, argument) {
        (Some(name), Some(value)) => Argument {
            name,
            value,
            span: Span::from(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible attribute arg during parsing: {:?}",
            token.as_str()
        ),
    }
}
