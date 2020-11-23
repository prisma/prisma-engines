mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use datamodel::{walkers::ModelWalker, walkers::ScalarFieldWalker, Datamodel, NativeTypeInstance, ScalarType};
use sql_schema_describer as sql;

pub(crate) trait SqlSchemaCalculatorFlavour {
    fn calculate_enums(&self, _datamodel: &Datamodel) -> Vec<sql::Enum> {
        Vec::new()
    }

    fn column_type_for_native_type(
        &self,
        field: &ScalarFieldWalker<'_>,
        scalar_type: ScalarType,
        native_type_instance: &NativeTypeInstance,
    ) -> sql::ColumnType;

    fn m2m_foreign_key_action(&self, _model_a: &ModelWalker<'_>, _model_b: &ModelWalker<'_>) -> sql::ForeignKeyAction {
        sql::ForeignKeyAction::Cascade
    }

    fn render_table_name(&self, original: &str) -> String {
        original.to_owned()
    }
}
