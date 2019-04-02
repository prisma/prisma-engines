use crate::ast::DatabaseValue;

#[derive(Debug, Clone, PartialEq)]
pub struct Count {
    pub(crate) exprs: Vec<DatabaseValue>,
}

/// Count of the underlying table where the given expression is not null.
///
/// ```rust
/// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
/// let query = Select::from_table("users").value(count(asterisk()));
/// let (sql, _) = Sqlite::build(query);
/// assert_eq!("SELECT COUNT(*) FROM `users` LIMIT -1", sql);
/// ```
#[inline]
pub fn count<T>(expr: T) -> Count
where
    T: Into<DatabaseValue>,
{
    Count {
        exprs: vec![expr.into()],
    }
}
