use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_expression::parse_arg_value,
    Rule,
};
use crate::ast::*;

pub fn parse_directive(token: &Token) -> Directive {
    let mut name: Option<Identifier> = None;
    let mut arguments: Vec<Argument> = vec![];

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::directive => return parse_directive(&current),
            Rule::directive_name => name = Some(current.to_id()),
            Rule::directive_arguments => parse_directive_args(&current, &mut arguments),
            _ => parsing_catch_all(&current, "directive"),
        }
    }

    match name {
        Some(name) => Directive {
            name,
            arguments,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!("Encountered impossible type during parsing: {:?}", token.as_str()),
    }
}

fn parse_directive_args(token: &Token, arguments: &mut Vec<Argument>) {
    for current in token.relevant_children() {
        match current.as_rule() {
            // This is a named arg.
            Rule::argument => arguments.push(parse_directive_arg(&current)),
            // This is a an unnamed arg.
            Rule::argument_value => arguments.push(Argument {
                name: Identifier::new(""),
                value: parse_arg_value(&current),
                span: Span::from_pest(current.as_span()),
            }),
            _ => parsing_catch_all(&current, "directive arguments"),
        }
    }
}

fn parse_directive_arg(token: &Token) -> Argument {
    let mut name: Option<Identifier> = None;
    let mut argument: Option<Expression> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::argument_name => name = Some(current.to_id()),
            Rule::argument_value => argument = Some(parse_arg_value(&current)),
            _ => parsing_catch_all(&current, "directive argument"),
        }
    }

    match (name, argument) {
        (Some(name), Some(value)) => Argument {
            name,
            value,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible directive arg during parsing: {:?}",
            token.as_str()
        ),
    }
}
