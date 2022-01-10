use super::SqlSchemaCalculatorFlavour;
use crate::flavour::MysqlFlavour;
use datamodel::{datamodel_connector::ScalarType, parser_database::walkers::*, ValidatedSchema};
use sql_schema_describer as sql;

impl SqlSchemaCalculatorFlavour for MysqlFlavour {
    fn calculate_enums(&self, datamodel: &ValidatedSchema<'_>) -> Vec<sql::Enum> {
        // This is a lower bound for the size of the generated enums (we assume
        // each enum is used at least once).
        let mut enums = Vec::new();

        let enum_fields = datamodel
            .db
            .walk_models()
            .flat_map(|model| model.scalar_fields())
            .filter_map(|field| field.field_type_as_enum().map(|enum_walker| (field, enum_walker)));

        for (field, enum_tpe) in enum_fields {
            let sql_enum = sql::Enum {
                name: format!(
                    "{model_name}_{field_name}",
                    model_name = field.model().final_database_name(),
                    field_name = field.database_name()
                ),
                values: enum_tpe.values().map(|v| v.database_name().to_owned()).collect(),
            };

            enums.push(sql_enum)
        }

        enums
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        sql_datamodel_connector::MYSQL.default_native_type_for_scalar_type(scalar_type)
    }

    fn enum_column_type(&self, field: ScalarFieldWalker<'_, '_>, _db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.ast_field().arity);

        sql::ColumnType::pure(
            sql::ColumnTypeFamily::Enum(format!(
                "{}_{}",
                field.model().final_database_name(),
                field.database_name()
            )),
            arity,
        )
    }
}
