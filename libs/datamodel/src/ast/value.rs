use super::*;

// TODO: Rename to expression.
/// Represents arbitrary, even nested, expressions.
#[derive(Debug, Clone)]
pub enum Value {
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
    Function(String, Vec<Value>, Span),
    /// An array of other values.
    Array(Vec<Value>, Span),
}

impl Value {
    pub fn with_lifted_span(&self, offset: usize) -> Value {
        match self {
            Value::NumericValue(v, s) => Value::NumericValue(v.clone(), lift_span(&s, offset)),
            Value::BooleanValue(v, s) => Value::BooleanValue(v.clone(), lift_span(&s, offset)),
            Value::StringValue(v, s) => Value::StringValue(v.clone(), lift_span(&s, offset)),
            Value::ConstantValue(v, s) => Value::ConstantValue(v.clone(), lift_span(&s, offset)),
            Value::Function(v, a, s) => Value::Function(
                v.clone(),
                a.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
                lift_span(&s, offset),
            ),
            Value::Array(v, s) => Value::Array(
                v.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
                lift_span(&s, offset),
            ),
            Value::Any(v, s) => Value::Any(v.clone(), lift_span(&s, offset)),
        }
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::StringValue(x, _) => x.clone(),
            Value::NumericValue(x, _) => x.clone(),
            Value::BooleanValue(x, _) => x.clone(),
            Value::ConstantValue(x, _) => x.clone(),
            Value::Function(x, _, _) => x.clone(),
            Value::Array(_, _) => String::from("(array)"),
            Value::Any(x, _) => x.clone(),
        }
    }
}

/// Creates a friendly readable representation for a value's type.
pub fn describe_value_type(val: &Value) -> &'static str {
    match val {
        Value::NumericValue(_, _) => "numeric",
        Value::BooleanValue(_, _) => "boolean",
        Value::StringValue(_, _) => "string",
        Value::ConstantValue(_, _) => "literal",
        Value::Function(_, _, _) => "functional",
        Value::Array(_, _) => "array",
        Value::Any(_, _) => "any",
    }
}
