use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

use super::Rule;
use crate::ast::*;

// Expressions

/// Parses an expression, given a Pest parser token.
pub fn parse_expression(token: &pest::iterators::Pair<'_, Rule>) -> Expression {
    return match_first! { token, current,
        Rule::numeric_literal => Expression::NumericValue(current.as_str().to_string(), Span::from_pest(current.as_span())),
        Rule::string_literal => Expression::StringValue(parse_string_literal(&current), Span::from_pest(current.as_span())),
        Rule::boolean_literal => Expression::BooleanValue(current.as_str().to_string(), Span::from_pest(current.as_span())),
        Rule::constant_literal => Expression::ConstantValue(current.as_str().to_string(), Span::from_pest(current.as_span())),
        Rule::function => parse_function(&current),
        Rule::array_expression => parse_array(&current),
        _ => unreachable!("Encountered impossible literal during parsing: {:?}", current.tokens())
    };
}

fn parse_function(token: &pest::iterators::Pair<'_, Rule>) -> Expression {
    let mut name: Option<String> = None;
    let mut arguments: Vec<Expression> = vec![];

    match_children! { token, current,
        Rule::non_empty_identifier => name = Some(current.as_str().to_string()),
        Rule::expression => arguments.push(parse_expression(&current)),
        _ => unreachable!("Encountered impossible function during parsing: {:?}", current.tokens())
    };

    match name {
        Some(name) => Expression::Function(name, arguments, Span::from_pest(token.as_span())),
        _ => unreachable!("Encountered impossible function during parsing: {:?}", token.as_str()),
    }
}

fn parse_array(token: &pest::iterators::Pair<'_, Rule>) -> Expression {
    let mut elements: Vec<Expression> = vec![];

    match_children! { token, current,
        Rule::expression => elements.push(parse_expression(&current)),
        _ => unreachable!("Encountered impossible array during parsing: {:?}", current.tokens())
    };

    Expression::Array(elements, Span::from_pest(token.as_span()))
}

pub fn parse_arg_value(token: &pest::iterators::Pair<'_, Rule>) -> Expression {
    match_first! { token, current,
        Rule::expression => parse_expression(&current),
        _ => unreachable!("Encountered impossible value during parsing: {:?}", current.tokens())
    }
}

fn parse_string_literal(token: &pest::iterators::Pair<'_, Rule>) -> String {
    return match_first! { token, current,
        Rule::string_content => unescape_string_literal(current.as_str()).into_owned(),
        _ => unreachable!("Encountered impossible string content during parsing: {:?}", current.tokens())
    };
}

fn unescape_string_literal(original: &str) -> Cow<'_, str> {
    const STRING_LITERAL_UNESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\(")"#).unwrap());
    const STRING_LITERAL_BACKSLASHES_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\\\"#).unwrap());
    const STRING_LITERAL_NEWLINE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\n"#).unwrap());

    match STRING_LITERAL_UNESCAPE_RE.replace_all(original, "\"") {
        Cow::Owned(s) => match STRING_LITERAL_NEWLINE_RE.replace_all(&s, "\n") {
            Cow::Owned(s) => STRING_LITERAL_BACKSLASHES_RE.replace_all(&s, "\\").into_owned().into(),
            Cow::Borrowed(s) => STRING_LITERAL_BACKSLASHES_RE.replace_all(s, "\\").into_owned().into(),
        },
        Cow::Borrowed(s) => match STRING_LITERAL_NEWLINE_RE.replace_all(s, "\n") {
            Cow::Owned(s) => STRING_LITERAL_BACKSLASHES_RE.replace_all(&s, "\\").into_owned().into(),
            Cow::Borrowed(s) => STRING_LITERAL_BACKSLASHES_RE.replace_all(s, "\\").into_owned().into(),
        },
    }
}
