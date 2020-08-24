use super::SqlSchemaCalculatorFlavour;
use crate::flavour::SqliteFlavour;

impl SqlSchemaCalculatorFlavour for SqliteFlavour {
    fn column_type_for_native_type(
        &self,
        _field: &datamodel::walkers::ScalarFieldWalker<'_>,
        _scalar_type: datamodel::ScalarType,
        _native_type_instance: &datamodel_connector::NativeTypeInstance,
    ) -> sql_schema_describer::ColumnType {
        unreachable!("column_type_for_native_type on SQLite")
    }
}
