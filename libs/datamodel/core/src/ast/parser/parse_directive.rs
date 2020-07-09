use super::{helpers::ToIdentifier, parse_expression::parse_arg_value, Rule};
use crate::ast::*;

pub fn parse_directive(token: &pest::iterators::Pair<'_, Rule>) -> Directive {
    let mut name: Option<Identifier> = None;
    let mut arguments: Vec<Argument> = vec![];

    match_children! { token, current,
        Rule::directive => return parse_directive(&current),
        Rule::directive_name => name = Some(current.to_id()),
        Rule::directive_arguments => parse_directive_args(&current, &mut arguments),
        _ => unreachable!("Encountered impossible directive during parsing: {:?} \n {:?}", token, current.tokens())
    };

    match name {
        Some(name) => Directive {
            name,
            arguments,
            span: Span::from_pest(token.as_span()),
        },
        _ => panic!("Encountered impossible type during parsing: {:?}", token.as_str()),
    }
}

fn parse_directive_args(token: &pest::iterators::Pair<'_, Rule>, arguments: &mut Vec<Argument>) {
    match_children! { token, current,
        // This is a named arg.
        Rule::argument => arguments.push(parse_directive_arg(&current)),
        // This is a an unnamed arg.
        Rule::argument_value => arguments.push(Argument {
            name: Identifier::new(""),
            value: parse_arg_value(&current),
            span: Span::from_pest(current.as_span())
        }),
        _ => unreachable!("Encountered impossible directive argument during parsing: {:?}", current.tokens())
    }
}

fn parse_directive_arg(token: &pest::iterators::Pair<'_, Rule>) -> Argument {
    let mut name: Option<Identifier> = None;
    let mut argument: Option<Expression> = None;

    match_children! { token, current,
        Rule::argument_name => name = Some(current.to_id()),
        Rule::argument_value => argument = Some(parse_arg_value(&current)),
        _ => unreachable!("Encountered impossible directive argument during parsing: {:?}", current.tokens())
    };

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
