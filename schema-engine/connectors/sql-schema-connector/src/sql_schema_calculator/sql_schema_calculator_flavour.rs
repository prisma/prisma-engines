use psl::parser_database::{ExtensionTypes, ast::FieldArity, walkers::*};
use sql_schema_describer::{self as sql, ColumnArity, ColumnType, ColumnTypeFamily};

pub(crate) trait SqlSchemaCalculatorFlavour {
    fn datamodel_connector(&self) -> &dyn psl::datamodel_connector::Connector;

    fn calculate_enums(&self, _ctx: &mut super::Context<'_>) {}

    fn calculate_extension_types(&self, _ctx: &mut super::Context<'_>, _extension_types: &dyn ExtensionTypes) {}

    fn column_type_for_enum(&self, _enm: EnumWalker<'_>, _ctx: &super::Context<'_>) -> Option<sql::ColumnTypeFamily> {
        None
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

    fn field_is_implicit_autoincrement_primary_key(&self, _field: ScalarFieldWalker<'_>) -> bool {
        false
    }

    fn m2m_foreign_key_action(&self, _model_a: ModelWalker<'_>, _model_b: ModelWalker<'_>) -> sql::ForeignKeyAction {
        sql::ForeignKeyAction::Cascade
    }

    fn push_connector_data(&self, _context: &mut super::Context<'_>) {}

    fn m2m_join_table_constraint(&self) -> JoinTableUniquenessConstraint {
        JoinTableUniquenessConstraint::UniqueIndex
    }

    fn normalize_index_predicate(&self, predicate: String, _is_raw: bool) -> String {
        predicate
    }
}

pub(crate) enum JoinTableUniquenessConstraint {
    PrimaryKey,
    UniqueIndex,
}
