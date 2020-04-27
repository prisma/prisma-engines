use crate::ast::*;

/// A builder for an `INSERT` statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Insert<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) columns: Vec<Column<'a>>,
    pub(crate) values: Vec<Row<'a>>,
    pub(crate) on_conflict: Option<OnConflict>,
    pub(crate) returning: Option<Vec<Column<'a>>>,
}

/// A builder for an `INSERT` statement for a single row.
pub struct SingleRowInsert<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) columns: Vec<Column<'a>>,
    pub(crate) values: Row<'a>,
}

/// A builder for an `INSERT` statement for multiple rows.
pub struct MultiRowInsert<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) columns: Vec<Column<'a>>,
    pub(crate) values: Vec<Row<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// `INSERT` conflict resolution strategies.
pub enum OnConflict {
    /// When a row already exists, do nothing.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query: Insert = Insert::single_into("users").into();
    ///
    /// let (sql, _) = Sqlite::build(query.on_conflict(OnConflict::DoNothing));
    ///
    /// assert_eq!("INSERT OR IGNORE INTO `users` DEFAULT VALUES", sql);
    /// ```
    DoNothing,
}

impl<'a> From<Insert<'a>> for Query<'a> {
    fn from(insert: Insert<'a>) -> Self {
        Query::Insert(Box::new(insert))
    }
}

impl<'a> From<SingleRowInsert<'a>> for Insert<'a> {
    fn from(insert: SingleRowInsert<'a>) -> Self {
        let values = if insert.values.is_empty() {
            Vec::new()
        } else {
            vec![insert.values]
        };

        Insert {
            table: insert.table,
            columns: insert.columns,
            values,
            on_conflict: None,
            returning: None,
        }
    }
}

impl<'a> From<MultiRowInsert<'a>> for Insert<'a> {
    fn from(insert: MultiRowInsert<'a>) -> Self {
        Insert {
            table: insert.table,
            columns: insert.columns,
            values: insert.values,
            on_conflict: None,
            returning: None,
        }
    }
}

impl<'a> From<SingleRowInsert<'a>> for Query<'a> {
    fn from(insert: SingleRowInsert<'a>) -> Query<'a> {
        Query::from(Insert::from(insert))
    }
}

impl<'a> From<MultiRowInsert<'a>> for Query<'a> {
    fn from(insert: MultiRowInsert<'a>) -> Query<'a> {
        Query::from(Insert::from(insert))
    }
}

impl<'a> Insert<'a> {
    /// Creates a new single row `INSERT` statement for the given table.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Insert::single_into("users");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("INSERT INTO `users` DEFAULT VALUES", sql);
    /// ```
    pub fn single_into<T>(table: T) -> SingleRowInsert<'a>
    where
        T: Into<Table<'a>>,
    {
        SingleRowInsert {
            table: table.into(),
            columns: Vec::new(),
            values: Row::new(),
        }
    }

    /// Creates a new multi row `INSERT` statement for the given table.
    pub fn multi_into<T, K, I>(table: T, columns: I) -> MultiRowInsert<'a>
    where
        T: Into<Table<'a>>,
        K: Into<Column<'a>>,
        I: IntoIterator<Item = K>,
    {
        MultiRowInsert {
            table: table.into(),
            columns: columns.into_iter().map(|c| c.into()).collect(),
            values: Vec::new(),
        }
    }

    /// Sets the conflict resolution strategy.
    pub fn on_conflict(mut self, on_conflict: OnConflict) -> Self {
        self.on_conflict = Some(on_conflict);
        self
    }

    /// Sets the returned columns.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Postgres}};
    /// let query = Insert::single_into("users");
    /// let insert = Insert::from(query).returning(vec!["id"]);
    /// let (sql, _) = Postgres::build(insert);
    ///
    /// assert_eq!("INSERT INTO \"users\" DEFAULT VALUES RETURNING \"id\"", sql);
    /// ```
    #[cfg(feature = "postgresql")]
    pub fn returning<K, I>(mut self, columns: I) -> Self
    where
        K: Into<Column<'a>>,
        I: IntoIterator<Item = K>,
    {
        self.returning = Some(columns.into_iter().map(|k| k.into()).collect());
        self
    }
}

impl<'a> SingleRowInsert<'a> {
    /// Adds a new value to the `INSERT` statement
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Insert::single_into("users").value("foo", 10);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("INSERT INTO `users` (`foo`) VALUES (?)", sql);
    /// assert_eq!(vec![Value::Integer(10)], params);
    /// ```
    pub fn value<K, V>(mut self, key: K, val: V) -> SingleRowInsert<'a>
    where
        K: Into<Column<'a>>,
        V: Into<Expression<'a>>,
    {
        self.columns.push(key.into());
        self.values.push(val.into());

        self
    }

    /// Convert into a common `Insert` statement.
    pub fn build(self) -> Insert<'a> {
        Insert::from(self)
    }
}

impl<'a> MultiRowInsert<'a> {
    /// Adds a new row to be inserted.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Insert::multi_into("users", vec!["foo"])
    ///     .values(vec![1])
    ///     .values(vec![2]);
    ///
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("INSERT INTO `users` (`foo`) VALUES (?), (?)", sql);
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::Integer(1),
    ///         Value::Integer(2),
    ///     ], params);
    /// ```
    pub fn values<V>(mut self, values: V) -> Self
    where
        V: Into<Row<'a>>,
    {
        self.values.push(values.into());
        self
    }

    /// Convert into a common `Insert` statement.
    pub fn build(self) -> Insert<'a> {
        Insert::from(self)
    }
}
