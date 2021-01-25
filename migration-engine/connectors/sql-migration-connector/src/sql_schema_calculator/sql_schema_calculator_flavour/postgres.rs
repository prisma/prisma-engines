use super::SqlSchemaCalculatorFlavour;
use crate::flavour::PostgresFlavour;
use datamodel::{walkers::ScalarFieldWalker, Datamodel, NativeTypeInstance, WithDatabaseName};
use native_types::PostgresType;
use sql::ColumnTypeFamily;
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

    fn default_native_type_for_family(&self, family: &ColumnTypeFamily) -> Option<serde_json::Value> {
        let ty = match family {
            ColumnTypeFamily::Int => PostgresType::Integer,
            ColumnTypeFamily::BigInt => PostgresType::BigInt,
            ColumnTypeFamily::Float => PostgresType::Decimal(Some((65, 30))),
            ColumnTypeFamily::Decimal => PostgresType::Decimal(Some((65, 30))),
            ColumnTypeFamily::Boolean => PostgresType::Boolean,
            ColumnTypeFamily::String => PostgresType::Text,
            ColumnTypeFamily::DateTime => PostgresType::Timestamp(Some(3)),
            ColumnTypeFamily::Binary => PostgresType::ByteA,
            ColumnTypeFamily::Json => PostgresType::JSONB,
            ColumnTypeFamily::Uuid => PostgresType::UUID,
            ColumnTypeFamily::Enum(_) => return None,
            ColumnTypeFamily::Unsupported(_) => return None,
        };

        Some(serde_json::to_value(ty).expect("PostgresType to json failed"))
    }

    fn column_type_for_native_type(
        &self,
        field: &ScalarFieldWalker<'_>,
        native_type_instance: &NativeTypeInstance,
    ) -> sql::ColumnType {
        let postgres_type: PostgresType = native_type_instance.deserialize_native_type();
        let is_autoincrement = field
            .default_value()
            .map(|default| default.is_autoincrement())
            .unwrap_or(false);

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

        let (family, data_type) = match postgres_type {
            PostgresType::SmallInt if is_autoincrement => (ColumnTypeFamily::Int, "SMALLSERIAL".to_owned()),
            PostgresType::SmallInt => (ColumnTypeFamily::Int, "SMALLINT".to_owned()),
            PostgresType::Integer if is_autoincrement => (ColumnTypeFamily::Int, "SERIAL".to_owned()),
            PostgresType::Integer => (ColumnTypeFamily::Int, "INTEGER".to_owned()),
            PostgresType::BigInt if is_autoincrement => (ColumnTypeFamily::BigInt, "BIGSERIAL".to_owned()),
            PostgresType::BigInt => (ColumnTypeFamily::BigInt, "BIGINT".to_owned()),
            PostgresType::Decimal(precision) => (
                ColumnTypeFamily::Decimal,
                format!("DECIMAL{}", render_decimal(precision)),
            ),
            PostgresType::Real => (ColumnTypeFamily::Float, "REAL".to_owned()),
            PostgresType::DoublePrecision => (ColumnTypeFamily::Float, "DOUBLE PRECISION".to_owned()),
            PostgresType::VarChar(length) => (ColumnTypeFamily::String, format!("VARCHAR{}", render(length))),
            PostgresType::Char(length) => (ColumnTypeFamily::String, format!("CHAR{}", render(length))),
            PostgresType::Text => (ColumnTypeFamily::String, "TEXT".to_owned()),
            PostgresType::ByteA => (ColumnTypeFamily::Binary, "BYTEA".to_owned()),
            PostgresType::Date => (ColumnTypeFamily::DateTime, "DATE".to_owned()),
            PostgresType::Timestamp(precision) => {
                (ColumnTypeFamily::DateTime, format!("TIMESTAMP{}", render(precision)))
            }
            PostgresType::Timestamptz(precision) => {
                (ColumnTypeFamily::DateTime, format!("TIMESTAMPTZ{}", render(precision)))
            }
            PostgresType::Time(precision) => (ColumnTypeFamily::DateTime, format!("TIME{}", render(precision))),
            PostgresType::Timetz(precision) => (ColumnTypeFamily::DateTime, format!("TIMETZ{}", render(precision))),
            PostgresType::Boolean => (ColumnTypeFamily::Boolean, "BOOLEAN".to_owned()),
            PostgresType::Bit(length) => (ColumnTypeFamily::String, format!("BIT{}", render(length))),
            PostgresType::VarBit(length) => (ColumnTypeFamily::String, format!("VARBIT{}", render(length))),
            PostgresType::UUID => (ColumnTypeFamily::Uuid, "UUID".to_owned()),
            PostgresType::Xml => (ColumnTypeFamily::String, "XML".to_owned()),
            PostgresType::JSON => (ColumnTypeFamily::Json, "JSON".to_owned()),
            PostgresType::JSONB => (ColumnTypeFamily::Json, "JSONB".to_owned()),
        };

        sql::ColumnType {
            full_data_type: data_type,
            family,
            arity: match field.arity() {
                datamodel::FieldArity::Required => sql::ColumnArity::Required,
                datamodel::FieldArity::Optional => sql::ColumnArity::Nullable,
                datamodel::FieldArity::List => sql::ColumnArity::List,
            },
            native_type: Some(native_type_instance.serialized_native_type.clone()),
        }
    }

    fn enum_column_type(&self, field: &ScalarFieldWalker<'_>, db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.arity());

        sql::ColumnType::pure(sql::ColumnTypeFamily::Enum(db_name.to_owned()), arity)
    }
}
