use crate::ast::*;
use std::borrow::Cow;

/// A builder for an `UPDATE` statement.
#[derive(Debug, PartialEq, Clone)]
pub struct Update<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) columns: Vec<Column<'a>>,
    pub(crate) values: Vec<Expression<'a>>,
    pub(crate) conditions: Option<ConditionTree<'a>>,
    pub(crate) comment: Option<Cow<'a, str>>,
    pub(crate) returning: Option<Vec<Column<'a>>>,
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
            comment: None,
            returning: None,
        }
    }

    /// Add another column value assignment to the query
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Update::table("users").set("foo", 10).set("bar", false);
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ?, `bar` = ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(10),
    ///         Value::from(false),
    ///     ],
    ///     params,
    /// );
    /// # Ok(())
    /// # }
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

    /// Adds a comment to the update.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let query = Update::table("users").set("foo", 10).comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");
    /// let (sql, _) = Sqlite::build(query)?;
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ? /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */", sql);
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
    /// let query = Update::table("users").set("foo", 1).so_that("bar".equals(false));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!("UPDATE `users` SET `foo` = ? WHERE `bar` = ?", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(1),
    ///         Value::from(false),
    ///     ],
    ///     params,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// We can also use a nested `SELECT` in the conditions.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let select = Select::from_table("bars").column("id").so_that("uniq_val".equals(3));
    /// let query = Update::table("users").set("foo", 1).so_that("bar".equals(select));
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!(
    ///     "UPDATE `users` SET `foo` = ? WHERE `bar` = (SELECT `id` FROM `bars` WHERE `uniq_val` = ?)",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(1),
    ///         Value::from(3),
    ///     ],
    ///     params,
    /// );
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

    /// Sets the returned columns.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Postgres}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let update = Update::table("users").set("foo", 10);
    /// let update = update.returning(vec!["id"]);
    /// let (sql, _) = Postgres::build(update)?;
    ///
    /// assert_eq!("UPDATE \"users\" SET \"foo\" = $1 RETURNING \"id\"", sql);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(any(feature = "postgresql", feature = "sqlite"))]
    pub fn returning<K, I>(mut self, columns: I) -> Self
    where
        K: Into<Column<'a>>,
        I: IntoIterator<Item = K>,
    {
        self.returning = Some(columns.into_iter().map(|k| k.into()).collect());
        self
    }
}
