use crate::ast::{self};

use super::{ExpressionPosition, WithSpan};

/// In an model attribute definition
#[derive(Debug)]
pub enum AttributePosition<'ast> {
    /// Nowhere specific inside the attribute (attribute name)
    Attribute,
    /// In an argument. (argument name)
    Argument(&'ast str),
    /// In an argument's value. (argument name, value)
    ArgumentValue(Option<&'ast str>, String),
    /// In an function argument. (function name, argument name, argument value)
    FunctionArgument(&'ast str, &'ast str, String),
}

impl<'ast> AttributePosition<'ast> {
    pub(crate) fn new(attr: &'ast ast::Attribute, position: usize) -> Self {
        let mut spans: Vec<(Option<&str>, ast::Span)> = attr
            .arguments
            .iter()
            .map(|arg| (arg.name.as_ref().map(|n| n.name.as_str()), arg.span()))
            .chain(
                attr.arguments
                    .empty_arguments
                    .iter()
                    .map(|arg| (Some(arg.name.name.as_str()), arg.name.span())),
            )
            .collect();

        spans.sort_by_key(|(_, span)| span.start);

        let mut arg_name = None;
        for (name, _) in spans.iter().take_while(|(_, span)| span.start < position) {
            arg_name = Some(*name);
        }

        // If the cursor is after a trailing comma, we're not in an argument.
        if let Some(span) = attr.arguments.trailing_comma {
            if position > span.start {
                arg_name = None;
            }
        }

        if let Some(arg) = attr.arguments.iter().find(|arg| arg.span().contains(position)) {
            if let ExpressionPosition::FunctionArgument(fun, name) = ExpressionPosition::new(&arg.value, position) {
                return Self::FunctionArgument(fun, name, arg.value.to_string());
            }

            if arg.value.is_array() {
                let arr = arg.value.as_array().unwrap();
                let expr = arr.0.iter().find(|expr| expr.span().contains(position));
                if let Some(expr) = expr {
                    return Self::ArgumentValue(arg.name(), expr.to_string());
                }
            }

            return Self::ArgumentValue(arg.name(), arg.value.to_string());

            // Self::ArgumentValue(arg_name, ())
        }

        if let Some(arg_name) = arg_name.flatten() {
            return Self::Argument(arg_name);
        }

        Self::Attribute
    }
}
