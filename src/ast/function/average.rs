use super::Function;
use crate::ast::Column;

#[derive(Debug, Clone, PartialEq)]
pub struct Average<'a> {
    pub(crate) column: Column<'a>,
}

/// Calculates the average value of a numeric column.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// let query = Select::from_table("users").value(avg("age"));
/// let (sql, _) = Sqlite::build(query);
/// assert_eq!("SELECT AVG(`age`) FROM `users`", sql);
/// ```
pub fn avg<'a, C>(col: C) -> Function<'a>
where
    C: Into<Column<'a>>,
{
    let fun = Average { column: col.into() };
    fun.into()
}
