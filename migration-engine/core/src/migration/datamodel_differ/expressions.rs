use datamodel::ast;

/// Compare two [AST expressions](/datamodel/ast/enum.Expression.html) semantically. We can't use a derived PartialEq because of the spans.
pub(crate) fn expressions_match(previous: &ast::Expression, next: &ast::Expression) -> bool {
    use ast::Expression;

    match (previous, next) {
        (Expression::Any(val1, _span1), Expression::Any(val2, _span2)) => val1 == val2,
        (Expression::Array(previous_values, _span1), Expression::Array(next_values, _span2)) => {
            previous_values.len() == next_values.len()
                && previous_values
                    .iter()
                    .zip(next_values.iter())
                    .all(|(previous_value, next_value)| expressions_match(previous_value, next_value))
        }
        (Expression::BooleanValue(val1, _span1), Expression::BooleanValue(val2, _span2)) => val1 == val2,
        (Expression::StringValue(val1, _span1), Expression::StringValue(val2, _span2)) => val1 == val2,
        (Expression::NumericValue(val1, _span1), Expression::NumericValue(val2, _span2)) => val1 == val2,
        (Expression::ConstantValue(val1, _span1), Expression::ConstantValue(val2, _span2)) => val1 == val2,
        (Expression::Function(name1, args1, _span1), Expression::Function(name2, args2, _span2)) => {
            name1 == name2
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(arg1, arg2)| expressions_match(arg1, arg2))
        }
        _ => false,
    }
}
