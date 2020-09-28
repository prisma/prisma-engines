mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use super::SqlSchemaCalculator;
use datamodel::{walkers::ScalarFieldWalker, ScalarType};
use datamodel_connector::NativeTypeInstance;
use sql_schema_describer as sql;

pub(crate) trait SqlSchemaCalculatorFlavour {
    fn calculate_enums(&self, _calculator: &SqlSchemaCalculator<'_>) -> Vec<sql::Enum> {
        Vec::new()
    }

    fn column_type_for_native_type(
        &self,
        _field: &ScalarFieldWalker<'_>,
        _scalar_type: ScalarType,
        _native_type_instance: &NativeTypeInstance,
    ) -> sql::ColumnType;
}
