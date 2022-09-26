use crate::ast::{self, Span};
use std::fmt;

/// Represents arbitrary, even nested, expressions.
#[derive(Debug, Clone)]
pub enum Expression {
    /// Any numeric value e.g. floats or ints.
    NumericValue(String, Span),
    /// Any string value.
    StringValue(String, Span),
    /// Any literal constant, basically a string which was not inside "...". This is used for
    /// representing builtin enums and boolean constants (true and false).
    ConstantValue(String, Span),
    /// A function call like node with a name and arguments.
    Function(String, ast::ArgumentsList, Span),
    /// An array of other values.
    Array(Vec<Expression>, Span),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::NumericValue(val, _) => fmt::Display::fmt(val, f),
            Expression::StringValue(val, _) => write!(f, "{}", crate::string_literal(val)),
            Expression::ConstantValue(val, _) => fmt::Display::fmt(val, f),
            Expression::Function(fun, args, _) => {
                let args = args.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
                write!(f, "{}({})", fun, args)
            }
            Expression::Array(vals, _) => {
                let vals = vals.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
                write!(f, "[{}]", vals)
            }
        }
    }
}

impl Expression {
    pub fn as_array(&self) -> Option<(&[Expression], Span)> {
        match self {
            Expression::Array(arr, span) => Some((arr, *span)),
            _ => None,
        }
    }

    pub fn as_string_value(&self) -> Option<(&str, Span)> {
        match self {
            Expression::StringValue(s, span) => Some((s, *span)),
            _ => None,
        }
    }

    pub fn as_constant_value(&self) -> Option<(&str, Span)> {
        match self {
            Expression::ConstantValue(s, span) => Some((s, *span)),
            _ => None,
        }
    }

    pub fn as_function(&self) -> Option<(&str, &ast::ArgumentsList, Span)> {
        match self {
            Expression::Function(name, args, span) => Some((name, args, *span)),
            _ => None,
        }
    }

    pub fn as_numeric_value(&self) -> Option<(&str, Span)> {
        match self {
            Expression::NumericValue(s, span) => Some((s, *span)),
            _ => None,
        }
    }

    pub fn span(&self) -> Span {
        match &self {
            Self::NumericValue(_, span) => *span,
            Self::StringValue(_, span) => *span,
            Self::ConstantValue(_, span) => *span,
            Self::Function(_, _, span) => *span,
            Self::Array(_, span) => *span,
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
            Expression::StringValue(_, _) => "string",
            Expression::ConstantValue(_, _) => "literal",
            Expression::Function(_, _, _) => "functional",
            Expression::Array(_, _) => "array",
        }
    }

    pub fn is_function(&self) -> bool {
        matches!(self, Expression::Function(_, _, _))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Expression::Array(_, _))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Expression::StringValue(_, _))
    }
}
