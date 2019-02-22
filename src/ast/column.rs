use crate::ast::{DatabaseValue, Table};

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

impl<'a> Into<Column> for &'a str {
    fn into(self) -> Column {
        Column {
            name: self.to_string(),
            table: None,
        }
    }
}

impl<'a, 'b> Into<Column> for (&'a str, &'b str) {
    fn into(self) -> Column {
        let mut column: Column = self.1.into();
        column = column.table(self.0.into());

        column
    }
}

impl<'a, 'b, 'c> Into<Column> for (&'a str, &'b str, &'c str) {
    fn into(self) -> Column {
        let column: Column = self.2.into();
        let table: Table = (self.0, self.1).into();

        column.table(table)
    }
}
