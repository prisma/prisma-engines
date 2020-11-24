use super::SqlSchemaCalculatorFlavour;
use crate::flavour::PostgresFlavour;
use datamodel::{walkers::ScalarFieldWalker, Datamodel, NativeTypeInstance, ScalarType, WithDatabaseName};
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

    fn column_type_for_native_type(
        &self,
        field: &ScalarFieldWalker<'_>,
        _scalar_type: ScalarType,
        native_type_instance: &NativeTypeInstance,
    ) -> sql::ColumnType {
        let postgres_type: PostgresType = native_type_instance.deserialize_native_type();

        fn render(input: Option<u32>) -> String {
            match input {
                None => "".to_string(),
                Some(arg) => format!("({})", arg),
            }
        }
        fn render_decimal(input: Option<(u32, u32)>) -> String {
            match input {
                None => "".to_string(),
                Some((precision, scale)) => format!("({}, {})", precision, scale),
            }
        }

        let data_type = match postgres_type {
            PostgresType::SmallInt => "SMALLINT".to_owned(),
            PostgresType::Integer => "INTEGER".to_owned(),
            PostgresType::BigInt => "BIGINT".to_owned(),
            PostgresType::Decimal(precision) => format!("DECIMAL{}", render_decimal(precision)),
            PostgresType::Numeric(precision) => format!("NUMERIC{}", render_decimal(precision)),
            PostgresType::Real => "REAL".to_owned(),
            PostgresType::DoublePrecision => "DOUBLE PRECISION".to_owned(),
            PostgresType::SmallSerial => "SMALLSERIAL".to_owned(),
            PostgresType::Serial => "SERIAL".to_owned(),
            PostgresType::BigSerial => "BIGSERIAL".to_owned(),
            PostgresType::VarChar(length) => format!("VARCHAR{}", render(length)),
            PostgresType::Char(length) => format!("CHAR{}", render(length)),
            PostgresType::Text => "TEXT".to_owned(),
            PostgresType::ByteA => "BYTEA".to_owned(),
            PostgresType::Timestamp(precision) => format!("TIMESTAMP{}", render(precision)),
            PostgresType::Timestamptz(precision) => format!("TIMESTAMPTZ{}", render(precision)),
            PostgresType::Date => "DATE".to_owned(),
            PostgresType::Time(precision) => format!("TIME{}", render(precision)),
            PostgresType::Timetz(precision) => format!("TIMETZ{}", render(precision)),
            PostgresType::Boolean => "BOOLEAN".to_owned(),
            PostgresType::Bit(length) => format!("BIT{}", render(length)),
            PostgresType::VarBit(length) => format!("VARBIT{}", render(length)),
            PostgresType::UUID => "UUID".to_owned(),
            PostgresType::Xml => "XML".to_owned(),
            PostgresType::JSON => "JSON".to_owned(),
            PostgresType::JSONB => "JSONB".to_owned(),
        };

        sql::ColumnType {
            data_type: data_type.clone(),
            full_data_type: data_type,
            character_maximum_length: None,
            family: sql::ColumnTypeFamily::String,
            arity: match field.arity() {
                datamodel::FieldArity::Required => sql::ColumnArity::Required,
                datamodel::FieldArity::Optional => sql::ColumnArity::Nullable,
                datamodel::FieldArity::List => sql::ColumnArity::List,
            },
            native_type: Some(native_type_instance.serialized_native_type.clone()),
        }
    }
}
