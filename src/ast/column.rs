use crate::ast::DatabaseValue;

/// A column definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    pub name: String,
    pub table: Option<String>,
    pub database: Option<String>,
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

    pub fn table<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.table = Some(name.into());
        self
    }

    pub fn database<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.database = Some(name.into());
        self
    }
}
