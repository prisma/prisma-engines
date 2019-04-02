use crate::ast::*;

#[derive(Debug, PartialEq, Clone)]
/// A builder for a `DELETE` statement.
pub struct Delete {
    pub(crate) table: Table,
    pub(crate) conditions: Option<ConditionTree>,
}

impl From<Delete> for Query {
    #[inline]
    fn from(delete: Delete) -> Query {
        Query::Delete(Box::new(delete))
    }
}

impl Delete {
    /// Creates a new `DELETE` statement for the given table.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Delete::from_table("users");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("DELETE FROM `users`", sql);
    /// ```
    #[inline]
    pub fn from_table<T>(table: T) -> Self
    where
        T: Into<Table>,
    {
        Self {
            table: table.into(),
            conditions: None,
        }
    }

    /// Adds `WHERE` conditions to the query. See
    /// [Comparable](trait.Comparable.html#required-methods) for more examples.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Delete::from_table("users").so_that("bar".equals(false));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("DELETE FROM `users` WHERE `bar` = ?", sql);
    /// assert_eq!(vec![ParameterizedValue::Boolean(false)], params);
    /// ```
    pub fn so_that<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree>,
    {
        self.conditions = Some(conditions.into());
        self
    }
}
