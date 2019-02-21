use crate::ast::DatabaseValue;

/// A column definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    name: Option<String>,
    table: Option<String>,
    database: Option<String>,
}

impl Into<DatabaseValue> for Column {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Column(self)
    }
}

impl Column {
    /// Create a column definition.
    #[inline]
    pub fn new<T: ToString>(name: T) -> Self {
        Column::default().name(name)
    }

    pub fn table<T: ToString>(mut self, name: T) -> Self {
        self.table = Some(name.to_string());
        self
    }

    pub fn database<T: ToString>(mut self, name: T) -> Self {
        self.database = Some(name.to_string());
        self
    }

    /// Set the name.
    pub fn name<T: ToString>(mut self, value: T) -> Self {
        self.name = Some(value.to_string());
        self
    }
}
