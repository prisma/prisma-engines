use datamodel::walkers::ScalarFieldWalker;
use sql_schema_describer::ColumnTypeFamily;

use super::SqlSchemaCalculatorFlavour;
use crate::flavour::SqliteFlavour;

impl SqlSchemaCalculatorFlavour for SqliteFlavour {
    fn column_type_for_native_type(
        &self,
        _field: &datamodel::walkers::ScalarFieldWalker<'_>,
        _native_type_instance: &datamodel::NativeTypeInstance,
    ) -> sql_schema_describer::ColumnType {
        unreachable!("column_type_for_native_type on SQLite")
    }

    fn default_native_type_for_family(&self, _family: &ColumnTypeFamily) -> Option<serde_json::Value> {
        None
    }

    // Integer primary keys on SQLite are automatically assigned the rowid, which means they are automatically autoincrementing.
    fn field_is_implicit_autoincrement_primary_key(&self, field: &ScalarFieldWalker<'_>) -> bool {
        field.is_id() && field.field_type().is_int()
    }
}
