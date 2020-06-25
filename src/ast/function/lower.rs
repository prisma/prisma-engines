use super::Function;
use crate::ast::Column;

#[derive(Debug, Clone, PartialEq)]
pub struct Lower<'a> {
    pub(crate) column: Column<'a>,
}

/// Compute the lowercased form of a string.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query =
/// Select::from_table("users").value(lower("username").alias("lower_username")).so_that("lower_username".equals("someuser"));
/// let (sql, _) = Sqlite::build(query)?;
/// assert_eq!("SELECT LOWER(`username`) AS `lower_username` FROM `users` WHERE `lower_username` = ?", sql);
/// # Ok(())
/// # }
/// ```
pub fn lower<'a, C>(col: C) -> Function<'a>
where
    C: Into<Column<'a>>,
{
    let fun = Lower { column: col.into() };

    fun.into()
}
