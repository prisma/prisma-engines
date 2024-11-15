use rusqlite::Column;

use crate::connector::{ColumnType, TypeIdentifier};

impl From<&Column<'_>> for ColumnType {
    fn from(value: &Column) -> Self {
        if value.is_float() {
            // Sqlite always returns Double for floats
            ColumnType::Double
        } else {
            ColumnType::from_type_identifier(value)
        }
    }
}
