use super::SqlSchemaCalculatorFlavour;
use crate::flavour::PostgresFlavour;
use datamodel::{walkers::ScalarFieldWalker, Datamodel, ScalarType, WithDatabaseName};
use native_types::PostgresType;
use sql_schema_describer::{self as sql};

impl SqlSchemaCalculatorFlavour for PostgresFlavour {
    fn calculate_enums(&self, datamodel: &Datamodel) -> Vec<sql::Enum> {
        datamodel
            .enums()
            .map(|r#enum| sql::Enum {
                name: r#enum.final_database_name().to_owned(),
                values: r#enum.database_values(),
            })
            .collect()
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let ty = match scalar_type {
            ScalarType::Int => PostgresType::Integer,
            ScalarType::BigInt => PostgresType::BigInt,
            ScalarType::Float => PostgresType::Decimal(Some((65, 30))),
            ScalarType::Decimal => PostgresType::Decimal(Some((65, 30))),
            ScalarType::Boolean => PostgresType::Boolean,
            ScalarType::String => PostgresType::Text,
            ScalarType::DateTime => PostgresType::Timestamp(Some(3)),
            ScalarType::Bytes => PostgresType::ByteA,
            ScalarType::Json => PostgresType::JSONB,
        };

        serde_json::to_value(ty).expect("PostgresType to json failed")
    }

    fn enum_column_type(&self, field: &ScalarFieldWalker<'_>, db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.arity());

        sql::ColumnType::pure(sql::ColumnTypeFamily::Enum(db_name.to_owned()), arity)
    }
}
