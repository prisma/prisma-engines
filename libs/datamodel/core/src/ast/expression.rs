use std::fmt;

use itertools::Itertools;

use super::*;

/// Represents arbitrary, even nested, expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Any numeric value e.g. floats or ints.
    NumericValue(String, Span),
    /// Any boolean value.
    BooleanValue(String, Span),
    /// Any string value.
    StringValue(String, Span),
    /// Any literal constant, basically a string which was not inside "...".
    /// This is used for representing builtin enums.
    ConstantValue(String, Span),
    /// A function with a name and arguments, which is evaluated at client side.
    Function(String, Vec<Expression>, Span),
    /// An array of other values.
    ExpressionArray(Vec<Expression>, Span),
    /// A contained list of arguments.
    ConstantValueWithArgs(String, Vec<Argument>, Span),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::NumericValue(val, _) => write!(f, "{}", val),
            Expression::BooleanValue(val, _) => write!(f, "{}", val),
            Expression::StringValue(val, _) => write!(f, "\"{}\"", val),
            Expression::ConstantValue(val, _) => write!(f, "{}", val),
            Expression::Function(fun, args, _) => {
                write!(f, "{}({})", fun, args.iter().map(|arg| format!("{}", arg)).join(","))
            }
            Expression::ExpressionArray(vals, _) => {
                write!(f, "[{}]", vals.iter().map(|arg| format!("{}", arg)).join(","))
            }
            Expression::ConstantValueWithArgs(ident, vals, _) => {
                write!(f, "{}({})", ident, vals.iter().map(|arg| format!("{}", arg)).join(","))
            }
        }
    }
}

impl Expression {
    pub fn as_string_value(&self) -> Option<(&str, Span)> {
        match self {
            Expression::StringValue(s, span) => Some((s, *span)),
            _ => None,
        }
    }

    // pub fn with_lifted_span(&self, offset: usize) -> Expression {
    //     match self {
    //         Expression::NumericValue(v, s) => Expression::NumericValue(v.clone(), s.lift_span(offset)),
    //         Expression::BooleanValue(v, s) => Expression::BooleanValue(v.clone(), s.lift_span(offset)),
    //         Expression::StringValue(v, s) => Expression::StringValue(v.clone(), s.lift_span(offset)),
    //         Expression::ConstantValue(v, s) => Expression::ConstantValue(v.clone(), s.lift_span(offset)),
    //         Expression::Function(v, a, s) => Expression::Function(
    //             v.clone(),
    //             a.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
    //             s.lift_span(offset),
    //         ),
    //         Expression::ExpressionArray(v, s) => Expression::ExpressionArray(
    //             v.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
    //             s.lift_span(offset),
    //         ),
    //         Expression::ArgumentArray(v, s) => Expression::ArgumentArray(
    //             v.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
    //             s.lift_span(offset),
    //         ),
    //     }
    // }

    pub fn render_to_string(&self) -> String {
        crate::ast::renderer::Renderer::render_value_to_string(self)
    }

    pub fn span(&self) -> Span {
        match &self {
            Self::NumericValue(_, span) => *span,
            Self::BooleanValue(_, span) => *span,
            Self::StringValue(_, span) => *span,
            Self::ConstantValue(_, span) => *span,
            Self::Function(_, _, span) => *span,
            Self::ExpressionArray(_, span) => *span,
            Self::ConstantValueWithArgs(_, _, span) => *span,
        }
    }

    pub fn is_env_expression(&self) -> bool {
        match &self {
            Self::Function(name, _, _) => name == "env",
            _ => false,
        }
    }

    /// Creates a friendly readable representation for a value's type.
    pub fn describe_value_type(&self) -> &'static str {
        match self {
            Expression::NumericValue(_, _) => "numeric",
            Expression::BooleanValue(_, _) => "boolean",
            Expression::StringValue(_, _) => "string",
            Expression::ConstantValue(_, _) => "literal",
            Expression::Function(_, _, _) => "functional",
            Expression::ExpressionArray(_, _) => "expression array",
            Expression::ConstantValueWithArgs(_, _, _) => "constant with args",
        }
    }
}
