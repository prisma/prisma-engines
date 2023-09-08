use super::Function;
use crate::ast::Expression;

/// A represention of the `LOWER` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct Lower<'a> {
    pub(crate) expression: Box<Expression<'a>>,
}

/// Converts the result of the expression into lowercase string.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(lower(Column::from("name")));
/// let (sql, _) = Sqlite::build(query)?;
/// assert_eq!("SELECT LOWER(`name`) FROM `users`", sql);
/// # Ok(())
/// # }
/// ```
pub fn lower<'a, E>(expression: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = Lower {
        expression: Box::new(expression.into()),
    };

    fun.into()
}
