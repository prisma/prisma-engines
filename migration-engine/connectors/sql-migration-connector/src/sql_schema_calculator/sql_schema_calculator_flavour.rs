mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use datamodel::{
    walkers::ModelWalker,
    walkers::{RelationFieldWalker, ScalarFieldWalker},
    Datamodel, FieldArity, ReferentialAction, ScalarType,
};
use sql_schema_describer::{self as sql, ColumnArity, ColumnType, ColumnTypeFamily};

pub(crate) trait SqlSchemaCalculatorFlavour {
    fn calculate_enums(&self, _datamodel: &Datamodel) -> Vec<sql::Enum> {
        Vec::new()
    }

    fn column_type_for_unsupported_type(&self, field: &ScalarFieldWalker<'_>, description: String) -> sql::ColumnType {
        ColumnType {
            full_data_type: description.clone(),
            family: ColumnTypeFamily::Unsupported(description),
            arity: match field.arity() {
                FieldArity::Required => ColumnArity::Required,
                FieldArity::Optional => ColumnArity::Nullable,
                FieldArity::List => ColumnArity::List,
            },
            native_type: None,
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value;

    fn enum_column_type(&self, _field: &ScalarFieldWalker<'_>, _db_name: &str) -> sql::ColumnType {
        unreachable!("unreachable enum_column_type")
    }

    fn field_is_implicit_autoincrement_primary_key(&self, _field: &ScalarFieldWalker<'_>) -> bool {
        false
    }

    fn on_update_action(&self, rf: &RelationFieldWalker<'_>) -> sql::ForeignKeyAction {
        let default = || match rf.arity() {
            FieldArity::Required => sql::ForeignKeyAction::Cascade,
            FieldArity::Optional => sql::ForeignKeyAction::SetNull,
            FieldArity::List => unreachable!(),
        };

        rf.on_update_action()
            .map(convert_referential_action)
            .unwrap_or_else(default)
    }

    fn on_delete_action(&self, rf: &RelationFieldWalker<'_>) -> sql::ForeignKeyAction {
        let default = || match rf.arity() {
            FieldArity::Required => sql::ForeignKeyAction::Restrict,
            FieldArity::Optional => sql::ForeignKeyAction::SetNull,
            FieldArity::List => unreachable!(),
        };

        rf.on_delete_action()
            .map(convert_referential_action)
            .unwrap_or_else(default)
    }

    fn m2m_foreign_key_action(&self, _model_a: &ModelWalker<'_>, _model_b: &ModelWalker<'_>) -> sql::ForeignKeyAction {
        sql::ForeignKeyAction::Cascade
    }

    // TODO: Maybe we should rethink this a bit?
    fn single_field_index_name(&self, model_name: &str, field_name: &str) -> String {
        format!("{}.{}_unique", model_name, field_name)
    }
}

fn convert_referential_action(action: ReferentialAction) -> sql::ForeignKeyAction {
    match action {
        ReferentialAction::Cascade => sql::ForeignKeyAction::Cascade,
        ReferentialAction::Restrict => sql::ForeignKeyAction::Restrict,
        ReferentialAction::NoAction => sql::ForeignKeyAction::NoAction,
        ReferentialAction::SetNull => sql::ForeignKeyAction::SetNull,
        ReferentialAction::SetDefault => sql::ForeignKeyAction::SetDefault,
    }
}
