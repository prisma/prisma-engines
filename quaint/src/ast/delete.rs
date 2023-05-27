use crate::ast::*;
use std::borrow::Cow;

#[derive(Debug, PartialEq, Clone)]
/// A builder for a `DELETE` statement.
pub struct Delete<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) conditions: Option<ConditionTree<'a>>,
    pub(crate) comment: Option<Cow<'a, str>>,
}

impl<'a> From<Delete<'a>> for Query<'a> {
    fn from(delete: Delete<'a>) -> Self {
        Query::Delete(Box::new(delete))
    }
}

impl<'a> Delete<'a> {
    /// Creates a new `DELETE` statement for the given table.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Delete::from_table("users");
    /// let (sql, _) = Sqlite::build(query)?;
    ///
    /// assert_eq!("DELETE FROM `users`", sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_table<T>(table: T) -> Self
    where
        T: Into<Table<'a>>,
    {
        Self {
            table: table.into(),
            conditions: None,
            comment: None,
        }
    }

    /// Adds a comment to the delete.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Delete::from_table("users").comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");
    /// let (sql, _) = Sqlite::build(query)?;
    ///
    /// assert_eq!("DELETE FROM `users` /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */", sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn comment<C: Into<Cow<'a, str>>>(mut self, comment: C) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Adds `WHERE` conditions to the query. See
    /// [Comparable](trait.Comparable.html#required-methods) for more examples.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Delete::from_table("users").so_that("bar".equals(false));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("DELETE FROM `users` WHERE `bar` = ?", sql);
    /// assert_eq!(vec![Value::boolean(false)], params);
    /// # Ok(())
    /// # }
    /// ```
    pub fn so_that<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree<'a>>,
    {
        self.conditions = Some(conditions.into());
        self
    }
}
