use super::Function;
use crate::ast::Expression;

#[derive(Debug, Clone, PartialEq)]
/// Returns the number of rows that matches a specified criteria.
pub struct Count<'a> {
    pub(crate) exprs: Vec<Expression<'a>>,
}

/// Count of the underlying table where the given expression is not null.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(count(asterisk()));
/// let (sql, _) = Sqlite::build(query)?;
/// assert_eq!("SELECT COUNT(*) FROM `users`", sql);
/// # Ok(())
/// # }
/// ```
pub fn count<'a, T>(expr: T) -> Function<'a>
where
    T: Into<Expression<'a>>,
{
    let fun = Count {
        exprs: vec![expr.into()],
    };

    fun.into()
}
