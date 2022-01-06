use super::{
    helpers::{parsing_catch_all, ToIdentifier, Token, TokenExtensions},
    parse_expression::parse_expression,
    Rule,
};
use crate::ast;

pub(crate) fn parse_arguments_list(token: &Token<'_>, arguments: &mut ast::ArgumentsList) {
    debug_assert_eq!(token.as_rule(), Rule::arguments_list);
    for current in token.relevant_children() {
        match current.as_rule() {
            // This is a named arg.
            Rule::named_argument => arguments.arguments.push(parse_named_arg(&current)),
            // This is an unnamed arg.
            Rule::expression => arguments.arguments.push(ast::Argument {
                name: None,
                value: parse_expression(&current),
                span: ast::Span::from(current.as_span()),
            }),
            // This is an argument without a value.
            // It is not valid, but we parse it for autocompletion.
            Rule::empty_argument => {
                let name = current
                    .into_inner()
                    .find(|tok| tok.as_rule() == Rule::argument_name)
                    .unwrap();
                arguments
                    .empty_arguments
                    .push(ast::EmptyArgument { name: name.to_id() })
            }
            Rule::trailing_comma => {
                arguments.trailing_comma = Some(current.as_span().into());
            }
            _ => parsing_catch_all(&current, "attribute arguments"),
        }
    }
}

fn parse_named_arg(token: &Token<'_>) -> ast::Argument {
    debug_assert_eq!(token.as_rule(), Rule::named_argument);
    let mut name: Option<ast::Identifier> = None;
    let mut argument: Option<ast::Expression> = None;

    for current in token.relevant_children() {
        match current.as_rule() {
            Rule::argument_name => name = Some(current.to_id()),
            Rule::expression => argument = Some(parse_expression(&current)),
            _ => parsing_catch_all(&current, "attribute argument"),
        }
    }

    match (name, argument) {
        (Some(name), Some(value)) => ast::Argument {
            name: Some(name),
            value,
            span: ast::Span::from(token.as_span()),
        },
        _ => panic!(
            "Encountered impossible attribute arg during parsing: {:?}",
            token.as_str()
        ),
    }
}
