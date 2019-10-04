use datamodel::ast;

/// Compare two [AST expressions](/datamodel/ast/enum.Value.html) semantically. We can't use a derived PartialEq because of the spans.
pub(crate) fn values_match(previous: &ast::Value, next: &ast::Value) -> bool {
    use ast::Value;

    match (previous, next) {
        (Value::Any(val1, _span1), Value::Any(val2, _span2)) => val1 == val2,
        (Value::Array(previous_values, _span1), Value::Array(next_values, _span2)) => {
            previous_values.len() == next_values.len()
                && previous_values
                    .iter()
                    .zip(next_values.iter())
                    .all(|(previous_value, next_value)| values_match(previous_value, next_value))
        }
        (Value::BooleanValue(val1, _span1), Value::BooleanValue(val2, _span2)) => val1 == val2,
        (Value::StringValue(val1, _span1), Value::StringValue(val2, _span2)) => val1 == val2,
        (Value::NumericValue(val1, _span1), Value::NumericValue(val2, _span2)) => val1 == val2,
        (Value::ConstantValue(val1, _span1), Value::ConstantValue(val2, _span2)) => val1 == val2,
        (Value::Function(name1, args1, _span1), Value::Function(name2, args2, _span2)) => {
            name1 == name2
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(arg1, arg2)| values_match(arg1, arg2))
        }
        _ => false,
    }
}
