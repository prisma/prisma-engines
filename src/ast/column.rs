use crate::ast::{Comparable, Compare, DatabaseValue, Table};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TypeIdentifier {
    String,
    Float,
    Boolean,
    Enum,
    Json,
    DateTime,
    GraphQLID,
    UUID,
    Int,
    Relation,
}

/// A column definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    pub name: String,
    pub table: Option<Table>,
    pub alias: Option<String>,
    pub type_identifier: Option<TypeIdentifier>,
}

impl Into<DatabaseValue> for Column {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Column(self)
    }
}

impl Column {
    /// Create a column definition.
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
    pub fn table(mut self, table: Table) -> Self {
        self.table = Some(table);
        self
    }

    /// Give the column an alias in the query.
    pub fn alias<S>(mut self, alias: S) -> Self
    where
        S: Into<String>,
    {
        self.alias = Some(alias.into());
        self
    }
}

impl<'a> From<&'a str> for Column {
    fn from(s: &'a str) -> Column {
        Column {
            name: s.to_string(),
            ..Default::default()
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
            ..Default::default()
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
    fn equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.equals(comparison)
    }

    fn not_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_equals(comparison)
    }

    fn less_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_equals(comparison)
    }

    fn less_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.less_than_or_equals(comparison)
    }

    fn greater_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.greater_than(comparison)
    }

    fn greater_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.greater_than_or_equals(comparison)
    }

    fn in_selection<T>(self, selection: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.in_selection(selection)
    }

    fn not_in_selection<T>(self, selection: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_in_selection(selection)
    }

    fn like<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.like(pattern)
    }

    fn not_like<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_like(pattern)
    }

    fn begins_with<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.begins_with(pattern)
    }

    fn not_begins_with<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_begins_with(pattern)
    }

    fn ends_into<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.ends_into(pattern)
    }

    fn not_ends_into<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_ends_into(pattern)
    }

    fn is_null(self) -> Compare {
        let value: DatabaseValue = self.into();
        value.is_null()
    }

    fn is_not_null(self) -> Compare {
        let value: DatabaseValue = self.into();
        value.is_not_null()
    }
}
