use crate::ast::*;

#[derive(Debug, PartialEq, Clone)]
pub struct PartialUpdate {
    pub(crate) table: Table,
}

impl PartialUpdate {
    /// Specify a value for the given column.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Update::table("users").set("foo", 10);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ?", sql);
    /// assert_eq!(vec![ParameterizedValue::Integer(10)], params);
    /// ```
    pub fn set<K, V>(self, column: K, value: V) -> Update
    where
        K: Into<Column>,
        V: Into<DatabaseValue>,
    {
        Update {
            table: self.table,
            columns: vec![column.into()],
            values: vec![value.into()],
            conditions: None,
            returning: None,
        }
    }
}

/// A builder for an `UPDATE` statement.
#[derive(Debug, PartialEq, Clone)]
pub struct Update {
    pub(crate) table: Table,
    pub(crate) columns: Vec<Column>,
    pub(crate) values: Vec<DatabaseValue>,
    pub(crate) conditions: Option<ConditionTree>,
    pub(crate) returning: Option<DatabaseValue>,
}

impl From<Update> for Query {
    #[inline]
    fn from(update: Update) -> Query {
        Query::Update(Box::new(update))
    }
}

impl Update {
    /// Creates the basis for an `UPDATE` statement to the given table.
    #[inline]
    pub fn table<T>(table: T) -> PartialUpdate
    where
        T: Into<Table>,
    {
        PartialUpdate {
            table: table.into(),
        }
    }

    /// Add another column value assignment to the query
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Update::table("users").set("foo", 10).set("bar", false);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ?, `bar` = ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         ParameterizedValue::Integer(10),
    ///         ParameterizedValue::Boolean(false)
    ///     ],
    ///     params,
    /// );
    /// ```
    pub fn set<K, V>(mut self, column: K, value: V) -> Update
    where
        K: Into<Column>,
        V: Into<DatabaseValue>,
    {
        self.columns.push(column.into());
        self.values.push(value.into());

        self
    }

    /// Adds `WHERE` conditions to the query. See
    /// [Comparable](trait.Comparable.html#required-methods) for more examples.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Update::table("users").set("foo", 1).so_that("bar".equals(false));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ? WHERE `bar` = ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         ParameterizedValue::Integer(1),
    ///         ParameterizedValue::Boolean(false)
    ///     ],
    ///     params,
    /// );
    /// ```
    ///
    /// We can also use a nested `SELECT` in the conditions.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let select = Select::from("bars").column("id").so_that("uniq_val".equals(3));
    /// let query = Update::table("users").set("foo", 1).so_that("bar".equals(select));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "UPDATE `users` SET `foo` = ? WHERE `bar` = (SELECT `id` FROM `bars` WHERE `uniq_val` = ? LIMIT -1)",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         ParameterizedValue::Integer(1),
    ///         ParameterizedValue::Integer(3)
    ///     ],
    ///     params,
    /// );
    /// ```
    pub fn so_that<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree>,
    {
        self.conditions = Some(conditions.into());
        self
    }

    /// Define the column(s) to be returned from the newly updated row.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Update::table("users").set("foo", 1).returning(asterisk());
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ? RETURNING *", sql);
    /// assert_eq!(vec![ParameterizedValue::Integer(1)], params);
    /// ```
    pub fn returning<T>(mut self, column: T) -> Self
    where
        T: Into<DatabaseValue>,
    {
        self.returning = Some(column.into());
        self
    }
}
