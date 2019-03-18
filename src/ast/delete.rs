use crate::ast::*;

#[derive(Debug, PartialEq, Clone)]
/// A builder for a `DELETE` statement.
pub struct Delete {
    pub(crate) table: Table,
    pub(crate) conditions: Option<ConditionTree>,
    pub(crate) returning: Option<DatabaseValue>,
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
    /// let query = Delete::from("users");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("DELETE FROM `users`", sql);
    /// ```
    #[inline]
    pub fn from<T>(table: T) -> Self
    where
        T: Into<Table>,
    {
        Self {
            table: table.into(),
            conditions: None,
            returning: None,
        }
    }

    /// Adds `WHERE` conditions to the query. See
    /// [Comparable](trait.Comparable.html#required-methods) for more examples.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Delete::from("users").so_that("bar".equals(false));
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

    /// Define the column(s) to be returned from the newly deleted row.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Delete::from("users").so_that("bar".equals(false)).returning(Column::from("id"));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("DELETE FROM `users` WHERE `bar` = ? RETURNING `id`", sql);
    /// assert_eq!(vec![ParameterizedValue::Boolean(false)], params);
    /// ```
    pub fn returning<T>(mut self, column: T) -> Self
    where
        T: Into<DatabaseValue>,
    {
        self.returning = Some(column.into());
        self
    }
}
