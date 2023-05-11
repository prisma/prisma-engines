use super::Function;
use crate::ast::Column;

/// A represention of the `MIN` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct Minimum<'a> {
    pub(crate) column: Column<'a>,
}

/// Calculates the minimum value of a numeric column.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(min("age"));
/// let (sql, _) = Sqlite::build(query)?;
/// assert_eq!("SELECT MIN(`age`) FROM `users`", sql);
/// # Ok(())
/// # }
/// ```
pub fn min<'a, C>(col: C) -> Function<'a>
where
    C: Into<Column<'a>>,
{
    let fun = Minimum { column: col.into() };
    fun.into()
}
