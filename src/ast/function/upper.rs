use super::Function;
use crate::ast::Expression;

/// A represention of the `UPPER` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct Upper<'a> {
    pub(crate) expression: Box<Expression<'a>>,
}

/// Converts the result of the expression into uppercase string.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(upper(Column::from("name")));
/// let (sql, _) = Sqlite::build(query)?;
/// assert_eq!("SELECT UPPER(`name`) FROM `users`", sql);
/// # Ok(())
/// # }
/// ```
pub fn upper<'a, E>(expression: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = Upper {
        expression: Box::new(expression.into()),
    };

    fun.into()
}
