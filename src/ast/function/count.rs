use crate::ast::DatabaseValue;

#[derive(Debug, Clone, PartialEq)]
pub struct Count {
    pub exprs: Vec<DatabaseValue>,
}

impl Count {
    /// Add another expression to a count statement
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let fun = count(Column::from(("users", "id")))
    ///     .count(Column::from(("users", "name")));
    ///
    /// let query = Select::from("users").value(fun);
    /// let (sql, _) = Sqlite::build(query);
    /// assert_eq!("SELECT COUNT(`users`.`id`, `users`.`name`) FROM `users` LIMIT -1", sql);
    /// ```
    pub fn count<T>(mut self, expr: T) -> Count
    where
        T: Into<DatabaseValue>,
    {
        self.exprs.push(expr.into());
        self
    }
}

/// Count of the underlying table where the given expression is not null.
///
/// ```rust
/// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
/// let query = Select::from("users").value(count(asterisk("users")));
/// let (sql, _) = Sqlite::build(query);
/// assert_eq!("SELECT COUNT(`users`.*) FROM `users` LIMIT -1", sql);
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
