use super::SqlSchemaCalculatorFlavour;
use crate::flavour::PostgresFlavour;
use datamodel::{datamodel_connector::ScalarType, parser_database::walkers::*, ValidatedSchema};
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

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        sql_datamodel_connector::POSTGRES.default_native_type_for_scalar_type(scalar_type)
    }

    fn enum_column_type(&self, field: ScalarFieldWalker<'_>, db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.ast_field().arity);

        sql::ColumnType::pure(sql::ColumnTypeFamily::Enum(db_name.to_owned()), arity)
    }
}
