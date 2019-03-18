use crate::ast::*;
use std::collections::BTreeMap;

/// A builder for an `INSERT` statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Insert {
    pub(crate) table: Table,
    pub(crate) values: BTreeMap<String, ParameterizedValue>,
    pub(crate) returning: Option<Column>,
}

impl From<Insert> for Query {
    #[inline]
    fn from(insert: Insert) -> Query {
        Query::Insert(Box::new(insert))
    }
}

impl Insert {
    /// Creates a new `INSERT` statement for the given table.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Insert::into("users");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("INSERT INTO `users` DEFAULT VALUES", sql);
    /// ```
    #[inline]
    pub fn into<T>(table: T) -> Self
    where
        T: Into<Table>,
    {
        Insert {
            table: table.into(),
            values: BTreeMap::new(),
            returning: None,
        }
    }

    /// Adds a new value to the `INSERT` statement
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Insert::into("users").value("foo", 10);
    /// let (sql, params) = Sqlite::build(query);
    ///
    /// assert_eq!("INSERT INTO `users` (`foo`) VALUES (?)", sql);
    /// assert_eq!(vec![ParameterizedValue::Integer(10)], params);
    /// ```
    pub fn value<K, V>(mut self, key: K, val: V) -> Self
    where
        K: Into<String>,
        V: Into<ParameterizedValue>,
    {
        self.values.insert(key.into(), val.into());
        self
    }

    /// Define the column to be returned from the newly inserted row.
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let query = Insert::into("users").returning("id");
    /// let (sql, _) = Sqlite::build(query);
    ///
    /// assert_eq!("INSERT INTO `users` DEFAULT VALUES RETURNING `id`", sql);
    /// ```
    pub fn returning<T>(mut self, column: T) -> Self
    where
        T: Into<Column>,
    {
        self.returning = Some(column.into());
        self
    }
}
