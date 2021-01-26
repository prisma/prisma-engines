use datamodel::{walkers::ScalarFieldWalker, NativeTypeInstance, ScalarType};

use super::SqlSchemaCalculatorFlavour;
use crate::flavour::SqliteFlavour;
use sql_schema_describer::ColumnType;

impl SqlSchemaCalculatorFlavour for SqliteFlavour {
    fn column_type_for_native_type(
        &self,
        _field: &ScalarFieldWalker<'_>,
        _native_type_instance: &NativeTypeInstance,
    ) -> ColumnType {
        unimplemented!("there are currently no native types on sqlite")
    }

    fn default_native_type_for_scalar_type(&self, _scalar_type: &ScalarType) -> serde_json::Value {
        serde_json::Value::Null
    }

    // Integer primary keys on SQLite are automatically assigned the rowid, which means they are automatically autoincrementing.
    fn field_is_implicit_autoincrement_primary_key(&self, field: &ScalarFieldWalker<'_>) -> bool {
        field.is_id() && field.field_type().is_int()
    }
}
