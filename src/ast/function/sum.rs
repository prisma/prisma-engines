use crate::ast::Column;

#[derive(Debug, Clone, PartialEq)]
pub struct Sum<'a> {
    pub(crate) column: Column<'a>,
}

/// Calculates the sum value of a numeric column.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// let query = Select::from_table("users").value(sum("age"));
/// let (sql, _) = Sqlite::build(query);
/// assert_eq!("SELECT SUM(`age`) FROM `users`", sql);
/// ```
#[inline]
pub fn sum<'a, C>(col: C) -> Sum<'a>
where
    C: Into<Column<'a>>,
{
    Sum { column: col.into() }
}
