mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use datamodel::{walkers::ModelWalker, walkers::ScalarFieldWalker, Datamodel, FieldArity, NativeTypeInstance};
use sql_schema_describer as sql;
use sql_schema_describer::{ColumnArity, ColumnType, ColumnTypeFamily};

pub(crate) trait SqlSchemaCalculatorFlavour {
    fn calculate_enums(&self, _datamodel: &Datamodel) -> Vec<sql::Enum> {
        Vec::new()
    }

    fn column_type_for_native_type(
        &self,
        field: &ScalarFieldWalker<'_>,
        native_type_instance: &NativeTypeInstance,
    ) -> sql::ColumnType;

    fn column_type_for_unsupported_type(&self, field: &ScalarFieldWalker<'_>, description: String) -> sql::ColumnType {
        ColumnType {
            full_data_type: description.clone(),
            family: ColumnTypeFamily::Unsupported(description.clone()),
            arity: match field.arity() {
                FieldArity::Required => ColumnArity::Required,
                FieldArity::Optional => ColumnArity::Nullable,
                FieldArity::List => ColumnArity::List,
            },
            native_type: None,
        }
    }

    fn default_native_type_for_family(&self, family: &sql::ColumnTypeFamily) -> Option<serde_json::Value>;

    fn enum_column_type(&self, _field: &ScalarFieldWalker<'_>, _db_name: &str) -> sql::ColumnType {
        unreachable!("unreachable enum_column_type")
    }

    fn field_is_implicit_autoincrement_primary_key(&self, _field: &ScalarFieldWalker<'_>) -> bool {
        false
    }

    fn m2m_foreign_key_action(&self, _model_a: &ModelWalker<'_>, _model_b: &ModelWalker<'_>) -> sql::ForeignKeyAction {
        sql::ForeignKeyAction::Cascade
    }

    // TODO: Maybe we should rethink this a bit?
    fn single_field_index_name(&self, model_name: &str, field_name: &str) -> String {
        format!("{}.{}_unique", model_name, field_name)
    }
}
