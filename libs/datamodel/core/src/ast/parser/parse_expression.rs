use super::helpers::{parsing_catch_all, Token, TokenExtensions};
use super::Rule;
use crate::ast::*;

pub fn parse_expression(token: &Token<'_>) -> Expression {
    let first_child = token.first_relevant_child();
    let span = Span::from_pest(first_child.as_span());
    match first_child.as_rule() {
        Rule::numeric_literal => Expression::NumericValue(first_child.as_str().to_string(), span),
        Rule::string_literal => Expression::StringValue(parse_string_literal(&first_child), span),
        Rule::boolean_literal => Expression::BooleanValue(first_child.as_str().to_string(), span),
        Rule::constant_literal => Expression::ConstantValue(first_child.as_str().to_string(), span),
        Rule::function => parse_function(&first_child),
        Rule::array_expression => parse_array(&first_child),
        _ => unreachable!(
            "Encountered impossible literal during parsing: {:?}",
            first_child.tokens()
        ),
    }
}

fn parse_function(token: &Token<'_>) -> Expression {
    let mut name: Option<String> = None;
    let mut arguments: Vec<Expression> = vec![];

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::non_empty_identifier => name = Some(current.as_str().to_string()),
            Rule::expression => arguments.push(parse_expression(&current)),
            _ => parsing_catch_all(&current, "function"),
        }
    }

    match name {
        Some(name) => Expression::Function(name, arguments, Span::from_pest(token.as_span())),
        _ => unreachable!("Encountered impossible function during parsing: {:?}", token.as_str()),
    }
}

fn parse_array(token: &Token<'_>) -> Expression {
    let mut elements: Vec<Expression> = vec![];

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::expression => elements.push(parse_expression(&current)),
            _ => parsing_catch_all(&current, "array"),
        }
    }

    Expression::Array(elements, Span::from_pest(token.as_span()))
}

pub fn parse_arg_value(token: &Token<'_>) -> Expression {
    let current = token.first_relevant_child();
    match current.as_rule() {
        Rule::expression => parse_expression(&current),
        _ => unreachable!("Encountered impossible value during parsing: {:?}", current.tokens()),
    }
}

fn parse_string_literal(token: &Token<'_>) -> String {
    let current = token.first_relevant_child();
    assert!(current.as_rule() == Rule::string_content);

    // this will overallocate a bit for strings with escaped characters, but it
    // shouldn't make a dramatic difference.
    let mut out = String::with_capacity(current.as_str().len());

    for pair in current.into_inner() {
        match pair.as_rule() {
            Rule::string_raw => {
                out.push_str(pair.as_str());
            }
            Rule::string_escape => {
                let escaped = pair.into_inner().next().unwrap();
                assert!(escaped.as_rule() == Rule::string_escaped_predefined);

                let unescaped = match escaped.as_str() {
                    "n" => "\n",
                    "r" => "\r",
                    "t" => "\t",
                    "0" => "\0",
                    other => other,
                };

                out.push_str(unescaped);
            }
            _ => unreachable!("Encountered impossible string content during parsing: {:?}", pair),
        }
    }

    out
}
