use super::SqlSchemaCalculatorFlavour;
use crate::flavour::MysqlFlavour;
use datamodel::{
    walkers::{walk_scalar_fields, ScalarFieldWalker},
    Datamodel, ScalarType,
};
use native_types::MySqlType;
use sql_schema_describer::{self as sql};

impl SqlSchemaCalculatorFlavour for MysqlFlavour {
    fn calculate_enums(&self, datamodel: &Datamodel) -> Vec<sql::Enum> {
        // This is a lower bound for the size of the generated enums (we assume
        // each enum is used at least once).
        let mut enums = Vec::with_capacity(datamodel.enums().len());

        let enum_fields = walk_scalar_fields(&datamodel)
            .filter_map(|field| field.field_type().as_enum().map(|enum_walker| (field, enum_walker)));

        for (field, enum_tpe) in enum_fields {
            let sql_enum = sql::Enum {
                name: format!(
                    "{model_name}_{field_name}",
                    model_name = field.model().database_name(),
                    field_name = field.db_name()
                ),
                values: enum_tpe.r#enum.database_values(),
            };

            enums.push(sql_enum)
        }

        enums
    }

    fn enum_column_type(&self, field: &ScalarFieldWalker<'_>, _db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.arity());

        sql::ColumnType::pure(
            sql::ColumnTypeFamily::Enum(format!("{}_{}", field.model().db_name(), field.db_name())),
            arity,
        )
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let ty = match scalar_type {
            ScalarType::Int => MySqlType::Int,
            ScalarType::BigInt => MySqlType::BigInt,
            ScalarType::Float => MySqlType::Decimal(Some((65, 30))),
            ScalarType::Decimal => MySqlType::Decimal(Some((65, 30))),
            ScalarType::Boolean => MySqlType::TinyInt,
            ScalarType::String => MySqlType::VarChar(191),
            ScalarType::DateTime => MySqlType::DateTime(Some(3)),
            ScalarType::Bytes => MySqlType::LongBlob,
            ScalarType::Json => MySqlType::Json,
        };

        serde_json::to_value(ty).expect("MySqlType to JSON failed")
    }
}
