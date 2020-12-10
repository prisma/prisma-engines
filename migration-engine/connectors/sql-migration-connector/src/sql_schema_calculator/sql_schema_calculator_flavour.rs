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

    fn enum_column_type(&self, _field: &ScalarFieldWalker<'_>, _db_name: &str) -> sql::ColumnType {
        unreachable!("unreachable enum_column_type")
    }

    fn field_is_implicit_autoincrement_primary_key(&self, _field: &ScalarFieldWalker<'_>) -> bool {
        false
    }

    fn m2m_foreign_key_action(&self, _model_a: &ModelWalker<'_>, _model_b: &ModelWalker<'_>) -> sql::ForeignKeyAction {
        sql::ForeignKeyAction::Cascade
    }

    /// returns whether the underlying databasae supports the RESTRICT setting for ON DELETE + ON UPDATE
    fn supports_foreign_key_restrict_constraint(&self) -> bool {
        true
    }

    // TODO: Maybe we should rethink this a bit?
    fn single_field_index_name(&self, model_name: &str, field_name: &str) -> String {
        format!("{}.{}_unique", model_name, field_name)
    }
}
