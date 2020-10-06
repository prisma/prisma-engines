use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_expression::parse_arg_value,
    Rule,
};
use crate::ast::*;

pub fn parse_attribute(token: &Token) -> Attribute {
    let mut name: Option<Identifier> = None;
    let mut arguments: Vec<Argument> = vec![];

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::attribute => return parse_attribute(&current),
            Rule::attribute_name => name = Some(current.to_id()),
            Rule::attribute_arguments => parse_attribute_args(&current, &mut arguments),
            _ => parsing_catch_all(&current, "attribute"),
        }
    }

    match name {
        Some(name) => Attribute {
            name,
            arguments,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!("Encountered impossible type during parsing: {:?}", token.as_str()),
    }
}

fn parse_attribute_args(token: &Token, arguments: &mut Vec<Argument>) {
    for current in token.relevant_children() {
        match current.as_rule() {
            // This is a named arg.
            Rule::argument => arguments.push(parse_attribute_arg(&current)),
            // This is a an unnamed arg.
            Rule::argument_value => arguments.push(Argument {
                name: Identifier::new(""),
                value: parse_arg_value(&current),
                span: Span::from_pest(current.as_span()),
            }),
            _ => parsing_catch_all(&current, "attribute arguments"),
        }
    }
}

fn parse_attribute_arg(token: &Token) -> Argument {
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
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible attribute arg during parsing: {:?}",
            token.as_str()
        ),
    }
}
