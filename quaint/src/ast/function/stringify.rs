use super::Function;
use crate::ast::Expression;

/// A represention of a string conversion or a casting function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct Stringify<'a> {
    pub(crate) expression: Box<Expression<'a>>,
}

/// Converts the expression into a string.
/// The exact semantics of this function depend on the database,
/// but it is generally used to convert non-string values into strings.
pub fn stringify<'a, E>(expression: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = Stringify {
        expression: Box::new(expression.into()),
    };

    fun.into()
}
