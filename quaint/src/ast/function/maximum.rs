use super::Function;
use crate::ast::Column;

/// A represention of the `MAX` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct Maximum<'a> {
    pub(crate) column: Column<'a>,
}

/// Calculates the maximum value of a numeric column.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(max("age"));
/// let (sql, _) = Sqlite::build(query)?;
/// assert_eq!("SELECT MAX(`age`) FROM `users`", sql);
/// # Ok(())
/// # }
/// ```
pub fn max<'a, C>(col: C) -> Function<'a>
where
    C: Into<Column<'a>>,
{
    let fun = Maximum { column: col.into() };
    fun.into()
}
