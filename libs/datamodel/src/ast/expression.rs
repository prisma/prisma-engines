use super::*;

/// Represents arbitrary, even nested, expressions.
#[derive(Debug, Clone)]
pub enum Expression {
    /// Any numeric value e.g. floats or ints.
    NumericValue(String, Span),
    /// Any boolean value.
    BooleanValue(String, Span),
    /// Any string value.
    StringValue(String, Span),
    /// A ducktyped string value, used as function return values which can be ducktyped.
    /// Canbe any scalar type, array or function is not possible.
    Any(String, Span),
    /// Any literal constant, basically a string which was not inside "...".
    /// This is used for representing builtin enums.
    ConstantValue(String, Span),
    /// A function with a name and arguments, which is evaluated at client side.
    Function(String, Vec<Expression>, Span),
    /// An array of other values.
    Array(Vec<Expression>, Span),
}

impl Expression {
    pub fn with_lifted_span(&self, offset: usize) -> Expression {
        match self {
            Expression::NumericValue(v, s) => Expression::NumericValue(v.clone(), lift_span(&s, offset)),
            Expression::BooleanValue(v, s) => Expression::BooleanValue(v.clone(), lift_span(&s, offset)),
            Expression::StringValue(v, s) => Expression::StringValue(v.clone(), lift_span(&s, offset)),
            Expression::ConstantValue(v, s) => Expression::ConstantValue(v.clone(), lift_span(&s, offset)),
            Expression::Function(v, a, s) => Expression::Function(
                v.clone(),
                a.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
                lift_span(&s, offset),
            ),
            Expression::Array(v, s) => Expression::Array(
                v.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
                lift_span(&s, offset),
            ),
            Expression::Any(v, s) => Expression::Any(v.clone(), lift_span(&s, offset)),
        }
    }
}

impl ToString for Expression {
    fn to_string(&self) -> String {
        match self {
            Expression::StringValue(x, _) => x.clone(),
            Expression::NumericValue(x, _) => x.clone(),
            Expression::BooleanValue(x, _) => x.clone(),
            Expression::ConstantValue(x, _) => x.clone(),
            Expression::Function(x, _, _) => x.clone(),
            Expression::Array(_, _) => String::from("(array)"),
            Expression::Any(x, _) => x.clone(),
        }
    }
}

/// Creates a friendly readable representation for a value's type.
pub fn describe_value_type(val: &Expression) -> &'static str {
    match val {
        Expression::NumericValue(_, _) => "numeric",
        Expression::BooleanValue(_, _) => "boolean",
        Expression::StringValue(_, _) => "string",
        Expression::ConstantValue(_, _) => "literal",
        Expression::Function(_, _, _) => "functional",
        Expression::Array(_, _) => "array",
        Expression::Any(_, _) => "any",
    }
}
