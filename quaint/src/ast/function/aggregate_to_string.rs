use super::Function;
use crate::ast::Expression;

#[derive(Debug, Clone, PartialEq)]
/// An aggregate function that concatenates strings from a group into a single
/// string with various options.
pub struct AggregateToString<'a> {
    pub(crate) value: Box<Expression<'a>>,
}

/// Aggregates the given field into a string.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(aggregate_to_string(Column::new("firstName")))
///     .group_by("firstName");
///
/// let (sql, _) = Sqlite::build(query)?;
/// assert_eq!("SELECT GROUP_CONCAT(`firstName`) FROM `users` GROUP BY `firstName`", sql);
/// # Ok(())
/// # }
/// ```
pub fn aggregate_to_string<'a, T>(expr: T) -> Function<'a>
where
    T: Into<Expression<'a>>,
{
    let fun = AggregateToString {
        value: Box::new(expr.into()),
    };

    fun.into()
}
