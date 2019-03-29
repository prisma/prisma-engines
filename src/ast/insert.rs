use crate::ast::*;

/// A builder for an `INSERT` statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Insert {
    pub(crate) table: Table,
    pub(crate) columns: Vec<Column>,
    pub(crate) values: Vec<Row>,
    pub(crate) on_conflict: Option<OnConflict>,
}

pub struct SingleRowInsert {
    pub(crate) table: Table,
    pub(crate) columns: Vec<Column>,
    pub(crate) values: Row,
}

pub struct MultiRowInsert {
    pub(crate) table: Table,
    pub(crate) columns: Vec<Column>,
    pub(crate) values: Vec<Row>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// `INSERT` conflict resolution strategies.
pub enum OnConflict {
    /// When a row already exists, do nothing.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query: Insert = Insert::single_into("users").into();
    ///
    /// let (sql, _) = Sqlite::build(query.on_conflict(OnConflict::DoNothing));
    ///
    /// assert_eq!("INSERT OR IGNORE INTO `users` DEFAULT VALUES", sql);
    /// ```
    DoNothing,
}

impl From<Insert> for Query {
    #[inline]
    fn from(insert: Insert) -> Query {
        Query::Insert(Box::new(insert))
    }
}

impl From<SingleRowInsert> for Insert {
    #[inline]
    fn from(insert: SingleRowInsert) -> Insert {
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
        }
    }
}

impl From<MultiRowInsert> for Insert {
    #[inline]
    fn from(insert: MultiRowInsert) -> Insert {
        Insert {
            table: insert.table,
            columns: insert.columns,
            values: insert.values,
            on_conflict: None,
        }
    }
}

impl From<SingleRowInsert> for Query {
    #[inline]
    fn from(insert: SingleRowInsert) -> Query {
        Query::from(Insert::from(insert))
    }
}

impl From<MultiRowInsert> for Query {
    #[inline]
    fn from(insert: MultiRowInsert) -> Query {
        Query::from(Insert::from(insert))
    }
}

impl Insert {
    /// Creates a new single row `INSERT` statement for the given table.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Insert::single_into("users");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("INSERT INTO `users` DEFAULT VALUES", sql);
    /// ```
    #[inline]
    pub fn single_into<T>(table: T) -> SingleRowInsert
    where
        T: Into<Table>,
    {
        let table: Table = table.into();

        SingleRowInsert {
            table: table,
            columns: Vec::new(),
            values: Row::new(),
        }
    }

    /// Creates a new multi row `INSERT` statement for the given table.
    #[inline]
    pub fn multi_into<T, K>(table: T, columns: Vec<K>) -> MultiRowInsert
    where
        T: Into<Table>,
        K: Into<Column>,
    {
        let table: Table = table.into();

        MultiRowInsert {
            table: table,
            columns: columns.into_iter().map(|c| c.into()).collect(),
            values: Vec::new(),
        }
    }

    /// Sets the conflict resolution strategy.
    #[inline]
    pub fn on_conflict(mut self, on_conflict: OnConflict) -> Self {
        self.on_conflict = Some(on_conflict);
        self
    }
}

impl SingleRowInsert {
    /// Adds a new value to the `INSERT` statement
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Insert::single_into("users").value("foo", 10);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("INSERT INTO `users` (`foo`) VALUES (?)", sql);
    /// assert_eq!(vec![ParameterizedValue::Integer(10)], params);
    /// ```
    pub fn value<K, V>(mut self, key: K, val: V) -> SingleRowInsert
    where
        K: Into<Column>,
        V: Into<DatabaseValue>,
    {
        self.columns.push(key.into());
        self.values = self.values.push(val.into());

        self
    }
}

impl MultiRowInsert {
    /// Adds a new row to be inserted.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
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
    ///         ParameterizedValue::Integer(1),
    ///         ParameterizedValue::Integer(2),
    ///     ], params);
    /// ```
    pub fn values<V>(mut self, values: V) -> Self
    where
        V: Into<Row>,
    {
        self.values.push(values.into());
        self
    }
}
