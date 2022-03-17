use super::SqlSchemaCalculatorFlavour;
use crate::flavour::PostgresFlavour;
use datamodel::{ast, datamodel_connector::ScalarType, parser_database::walkers::*, ValidatedSchema};
use sql_schema_describer as sql;

impl SqlSchemaCalculatorFlavour for PostgresFlavour {
    fn calculate_enums(&self, datamodel: &ValidatedSchema) -> Vec<sql::Enum> {
        datamodel
            .db
            .walk_enums()
            .map(|r#enum| sql::Enum {
                name: r#enum.database_name().to_owned(),
                values: r#enum.values().map(|val| val.database_name().to_owned()).collect(),
            })
            .collect()
    }

    fn column_type_for_unsupported_type(&self, field: ScalarFieldWalker<'_>, description: String) -> sql::ColumnType {
        sql::ColumnType {
            full_data_type: description.clone(),
            family: sql::ColumnTypeFamily::Unsupported(description),
            arity: match field.ast_field().arity {
                ast::FieldArity::Required => sql::ColumnArity::Required,
                ast::FieldArity::Optional => sql::ColumnArity::Nullable,
                ast::FieldArity::List => sql::ColumnArity::List,
            },
            native_type: None,
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        sql_datamodel_connector::POSTGRES.default_native_type_for_scalar_type(scalar_type)
    }

    fn enum_column_type(&self, field: ScalarFieldWalker<'_>, db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.ast_field().arity);

        sql::ColumnType::pure(sql::ColumnTypeFamily::Enum(db_name.to_owned()), arity)
    }

    fn field_is_implicit_autoincrement_primary_key(&self, field: ScalarFieldWalker<'_>) -> bool {
        if !self.is_cockroachdb() {
            return false;
        }

        let default = match field.default_value() {
            Some(default) => default,
            None => return false,
        };

        match default.value() {
            ast::Expression::Function(_, args, _) => {
                match args.arguments.first().and_then(|a| a.value.as_string_value()) {
                    Some((val, _)) => val == "unique_rowid()",
                    None => false,
                }
            }
            _ => false,
        }
    }

    fn m2m_foreign_key_action(&self, _model_a: ModelWalker<'_>, _model_b: ModelWalker<'_>) -> sql::ForeignKeyAction {
        sql::ForeignKeyAction::Cascade
    }
}
