use super::SqlSchemaCalculatorFlavour;
use crate::flavour::MysqlFlavour;
use datamodel::{parser_database::walkers::*, ValidatedSchema};
use sql_schema_describer as sql;

impl SqlSchemaCalculatorFlavour for MysqlFlavour {
    fn calculate_enums(&self, datamodel: &ValidatedSchema) -> Vec<sql::Enum> {
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
                    model_name = field.model().database_name(),
                    field_name = field.database_name()
                ),
                values: enum_tpe.values().map(|v| v.database_name().to_owned()).collect(),
            };

            enums.push(sql_enum)
        }

        enums
    }

    fn enum_column_type(&self, field: ScalarFieldWalker<'_>, _db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.ast_field().arity);

        sql::ColumnType::pure(
            sql::ColumnTypeFamily::Enum(format!("{}_{}", field.model().database_name(), field.database_name())),
            arity,
        )
    }
}
