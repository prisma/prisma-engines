mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use datamodel::{
    parser_database::{ast::FieldArity, walkers::*},
    ValidatedSchema,
};
use sql_schema_describer::{self as sql, ColumnArity, ColumnType, ColumnTypeFamily};

pub(crate) trait SqlSchemaCalculatorFlavour {
    fn calculate_enums(&self, _datamodel: &ValidatedSchema) -> Vec<sql::Enum> {
        Vec::new()
    }

    fn column_default_value_for_autoincrement(&self) -> Option<sql::DefaultValue> {
        None
    }

    fn column_type_for_unsupported_type(&self, field: ScalarFieldWalker<'_>, description: String) -> sql::ColumnType {
        ColumnType {
            full_data_type: description.clone(),
            family: ColumnTypeFamily::Unsupported(description),
            arity: match field.ast_field().arity {
                FieldArity::Required => ColumnArity::Required,
                FieldArity::Optional => ColumnArity::Nullable,
                FieldArity::List => ColumnArity::List,
            },
            native_type: None,
        }
    }

    fn default_constraint_name(&self, _default_value: DefaultValueWalker<'_>) -> Option<String> {
        None
    }

    fn enum_column_type(&self, _field: ScalarFieldWalker<'_>, _db_name: &str) -> sql::ColumnType {
        unreachable!("unreachable enum_column_type")
    }

    fn field_is_implicit_autoincrement_primary_key(&self, _field: ScalarFieldWalker<'_>) -> bool {
        false
    }

    fn m2m_foreign_key_action(&self, _model_a: ModelWalker<'_>, _model_b: ModelWalker<'_>) -> sql::ForeignKeyAction {
        sql::ForeignKeyAction::Cascade
    }

    fn push_connector_data(&self, _context: &mut super::Context<'_>) {}
}
