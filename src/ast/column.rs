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
