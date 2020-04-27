use crate::ast::*;

/// A builder for an `UPDATE` statement.
#[derive(Debug, PartialEq, Clone)]
pub struct Update<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) columns: Vec<Column<'a>>,
    pub(crate) values: Vec<Expression<'a>>,
    pub(crate) conditions: Option<ConditionTree<'a>>,
}

impl<'a> From<Update<'a>> for Query<'a> {
    fn from(update: Update<'a>) -> Self {
        Query::Update(Box::new(update))
    }
}

impl<'a> Update<'a> {
    /// Creates the basis for an `UPDATE` statement to the given table.
    pub fn table<T>(table: T) -> Self
    where
        T: Into<Table<'a>>,
    {
        Self {
            table: table.into(),
            columns: Vec::new(),
            values: Vec::new(),
            conditions: None,
        }
    }

    /// Add another column value assignment to the query
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Update::table("users").set("foo", 10).set("bar", false);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ?, `bar` = ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::Integer(10),
    ///         Value::Boolean(false),
    ///     ],
    ///     params,
    /// );
    /// ```
    pub fn set<K, V>(mut self, column: K, value: V) -> Update<'a>
    where
        K: Into<Column<'a>>,
        V: Into<Expression<'a>>,
    {
        self.columns.push(column.into());
        self.values.push(value.into());

        self
    }

    /// Adds `WHERE` conditions to the query. See
    /// [Comparable](trait.Comparable.html#required-methods) for more examples.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Update::table("users").set("foo", 1).so_that("bar".equals(false));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ? WHERE `bar` = ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::Integer(1),
    ///         Value::Boolean(false),
    ///     ],
    ///     params,
    /// );
    /// ```
    ///
    /// We can also use a nested `SELECT` in the conditions.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let select = Select::from_table("bars").column("id").so_that("uniq_val".equals(3));
    /// let query = Update::table("users").set("foo", 1).so_that("bar".equals(select));
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!(
    ///     "UPDATE `users` SET `foo` = ? WHERE `bar` = (SELECT `id` FROM `bars` WHERE `uniq_val` = ?)",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::Integer(1),
    ///         Value::Integer(3),
    ///     ],
    ///     params,
    /// );
    /// ```
    pub fn so_that<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree<'a>>,
    {
        self.conditions = Some(conditions.into());
        self
    }
}
