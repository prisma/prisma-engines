use crate::ast::*;

/// A builder for an `INSERT` statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Insert {
    pub(crate) table: Table,
    pub(crate) columns: Vec<Column>,
    pub(crate) values: Vec<DatabaseValue>,
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
        let table: Table = table.into();

        Insert {
            table: table,
            columns: Vec::new(),
            values: Vec::new(),
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
        K: Into<Column>,
        V: Into<DatabaseValue>,
    {
        self.columns.push(key.into());
        self.values.push(val.into());

        self
    }
}
