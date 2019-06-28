use crate::ast::DatabaseValue;

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateToString<'a> {
    pub(crate) value: Box<DatabaseValue<'a>>,
}

/// Aggregates the given field into a string.
///
/// ```rust
/// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
/// let query = Select::from_table("users").value(aggregate_to_string(Column::new("firstName")))
///     .group_by("firstName");
/// let (sql, _) = Sqlite::build(query);
/// assert_eq!("SELECT group_concat(`firstName`) FROM `users` GROUP BY `firstName`", sql);
/// ```
#[inline]
pub fn aggregate_to_string<'a, T>(expr: T) -> AggregateToString<'a>
where
    T: Into<DatabaseValue<'a>>,
{
    AggregateToString {
        value: Box::new(expr.into()),
    }
}
