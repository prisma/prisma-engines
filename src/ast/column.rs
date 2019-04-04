use crate::ast::{DatabaseValue, Table};

/// A column definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    pub name: String,
    pub(crate) table: Option<Table>,
    pub(crate) alias: Option<String>,
}

impl Into<DatabaseValue> for Column {
    #[inline]
    fn into(self) -> DatabaseValue {
        DatabaseValue::Column(Box::new(self))
    }
}

impl Column {
    /// Create a column definition.
    #[inline]
    pub fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Column {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Include the table name in the column expression.
    #[inline]
    pub fn table<T>(mut self, table: T) -> Self
    where
        T: Into<Table>,
    {
        self.table = Some(table.into());
        self
    }

    /// Include the table name in the column expression, if table is defined.
    #[inline]
    pub fn opt_table<T>(mut self, table: Option<T>) -> Self
    where
        T: Into<Table>,
    {
        if let Some(table) = table {
            self.table = Some(table.into());
        }

        self
    }

    /// Give the column an alias in the query.
    #[inline]
    pub fn alias<S>(mut self, alias: S) -> Self
    where
        S: Into<String>,
    {
        self.alias = Some(alias.into());
        self
    }
}

impl<'a> From<&'a str> for Column {
    #[inline]
    fn from(s: &'a str) -> Column {
        Column {
            name: s.to_string(),
            ..Default::default()
        }
    }
}

impl From<String> for Column {
    #[inline]
    fn from(s: String) -> Column {
        Column {
            name: s,
            ..Default::default()
        }
    }
}

impl<T, C> From<(T, C)> for Column
where
    T: Into<Table>,
    C: Into<Column>,
{
    #[inline]
    fn from(t: (T, C)) -> Column {
        let mut column: Column = t.1.into();
        column = column.table(t.0);

        column
    }
}
