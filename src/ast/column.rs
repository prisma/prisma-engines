use crate::ast::{Comparable, Compare, DatabaseValue, Table};

/// A column definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    pub name: String,
    pub table: Option<Table>,
}

impl Into<DatabaseValue> for Column {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Column(self)
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

    pub fn table(mut self, table: Table) -> Self {
        self.table = Some(table);
        self
    }
}

impl<'a> From<&'a str> for Column {
    fn from(s: &'a str) -> Column {
        Column {
            name: s.to_string(),
            table: None,
        }
    }
}

impl<'a, 'b> From<(&'a str, &'b str)> for Column {
    fn from(t: (&'a str, &'b str)) -> Column {
        let mut column: Column = t.1.into();
        column = column.table(t.0.into());

        column
    }
}

impl<'a, 'b, 'c> From<(&'a str, &'b str, &'c str)> for Column {
    fn from(t: (&'a str, &'b str, &'c str)) -> Column {
        let column: Column = t.2.into();
        let table: Table = (t.0, t.1).into();

        column.table(table)
    }
}

impl From<String> for Column {
    fn from(s: String) -> Column {
        Column {
            name: s,
            table: None,
        }
    }
}

impl From<(String, String)> for Column {
    fn from(s: (String, String)) -> Column {
        let mut column: Column = s.1.into();
        column = column.table(s.0.into());

        column
    }
}

impl From<(String, String, String)> for Column {
    fn from(s: (String, String, String)) -> Column {
        let column: Column = s.2.into();
        let table: Table = (s.0, s.1).into();

        column.table(table)
    }
}

impl Comparable for Column {
    #[inline]
    fn equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.equals(comparison)
    }

    #[inline]
    fn not_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_equals(comparison)
    }

    #[inline]
    fn less_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_equals(comparison)
    }

    #[inline]
    fn less_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.less_than_or_equals(comparison)
    }

    #[inline]
    fn greater_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.greater_than(comparison)
    }

    #[inline]
    fn greater_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.greater_than_or_equals(comparison)
    }

    #[inline]
    fn in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.in_selection(selection)
    }

    #[inline]
    fn not_in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_in_selection(selection)
    }

    #[inline]
    fn like<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.like(pattern)
    }

    #[inline]
    fn not_like<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_like(pattern)
    }

    #[inline]
    fn begins_with<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.begins_with(pattern)
    }

    #[inline]
    fn not_begins_with<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_begins_with(pattern)
    }

    #[inline]
    fn ends_into<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.ends_into(pattern)
    }

    #[inline]
    fn not_ends_into<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_ends_into(pattern)
    }

    #[inline]
    fn is_null(self) -> Compare {
        let value: DatabaseValue = self.into();
        value.is_null()
    }

    #[inline]
    fn is_not_null(self) -> Compare {
        let value: DatabaseValue = self.into();
        value.is_not_null()
    }
}
