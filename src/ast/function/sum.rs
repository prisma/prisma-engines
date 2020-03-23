use super::Function;
use crate::ast::Column;

#[derive(Debug, Clone, PartialEq)]
pub struct Sum<'a> {
    pub(crate) column: Column<'a>,
}

/// Calculates the sum value of a numeric column.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// let query = Select::from_table("users").value(sum("age").alias("sum"));
/// let (sql, _) = Sqlite::build(query);
/// assert_eq!("SELECT SUM(`age`) AS `sum` FROM `users`", sql);
/// ```
pub fn sum<'a, C>(col: C) -> Function<'a>
where
    C: Into<Column<'a>>,
{
    let fun = Sum { column: col.into() };

    fun.into()
}
