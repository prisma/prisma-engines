use diagnostics::Span;

use crate::ast::{self};

use super::{ExpressionPosition, WithName, WithSpan};

/// In an model attribute definition
#[derive(Debug)]
pub enum AttributePosition<'ast> {
    /// Nowhere specific inside the attribute (attribute name)
    Attribute,
    /// In an argument. (argument name)
    Argument(&'ast str, Span),
    /// In the argument's name. (argument name)
    ArgumentName(&'ast str, Span),
    /// In an argument's value. (argument name, value)
    ArgumentValue(Option<&'ast str>, String, Span),
    /// In an function argument. (function name, argument name, argument value)
    FunctionArgument(&'ast str, &'ast str, String, Span),
}

impl<'ast> AttributePosition<'ast> {
    pub(crate) fn new(attr: &'ast ast::Attribute, position: usize) -> Self {
        if let Some(arg) = attr.arguments.iter().find(|arg| arg.span().contains(position)) {
            if let ExpressionPosition::FunctionArgument(fun, name) = ExpressionPosition::new(&arg.value, position) {
                return Self::FunctionArgument(fun, name, arg.value.to_string(), arg.span);
            }

            if let Some(name) = &arg.name {
                if name.span.contains(position) {
                    return Self::ArgumentName(&name.name, name.span);
                }
            }

            if arg.value.is_array() {
                let arr = arg.value.as_array().unwrap();
                let expr = arr.0.iter().find(|expr| expr.span().contains(position));
                if let Some(expr) = expr {
                    return Self::ArgumentValue(arg.name(), expr.to_string(), expr.span());
                }
            }

            return Self::ArgumentValue(arg.name(), arg.value.to_string(), arg.value.span());
        }

        if let Some(arg) = attr
            .arguments
            .empty_arguments
            .iter()
            .find(|arg| arg.span().contains(position))
        {
            if arg.name.span.contains(position) {
                return Self::ArgumentName(arg.name(), arg.name.span());
            }

            return Self::Argument(arg.name(), arg.span());
        }

        Self::Attribute
    }
}
