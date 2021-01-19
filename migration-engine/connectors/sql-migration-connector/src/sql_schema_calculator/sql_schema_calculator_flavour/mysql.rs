use super::SqlSchemaCalculatorFlavour;
use crate::flavour::MysqlFlavour;
use datamodel::{
    walkers::{walk_scalar_fields, ScalarFieldWalker},
    Datamodel, NativeTypeInstance, ScalarType,
};
use native_types::MySqlType;
use sql::ColumnTypeFamily;
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

    fn column_type_for_native_type(
        &self,
        field: &ScalarFieldWalker<'_>,
        _scalar_type: ScalarType,
        native_type_instance: &NativeTypeInstance,
    ) -> sql::ColumnType {
        let mysql_type: MySqlType = native_type_instance.deserialize_native_type();

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

        let (family, data_type) = match mysql_type {
            MySqlType::Int => (ColumnTypeFamily::Int, "INTEGER".into()),
            MySqlType::SmallInt => (ColumnTypeFamily::Int, "SMALLINT".into()),
            MySqlType::TinyInt => (ColumnTypeFamily::Int, "TINYINT".into()),
            MySqlType::MediumInt => (ColumnTypeFamily::Int, "MEDIUMINT".into()),
            MySqlType::BigInt => (ColumnTypeFamily::BigInt, "BIGINT".into()),
            MySqlType::Decimal(precision) => (
                ColumnTypeFamily::Decimal,
                format!("DECIMAL{}", render_decimal(precision)),
            ),
            MySqlType::Float => (ColumnTypeFamily::Float, "FLOAT".into()),
            MySqlType::Double => (ColumnTypeFamily::Float, "DOUBLE".into()),
            MySqlType::Bit(size) => (ColumnTypeFamily::Binary, format!("BIT({size})", size = size)),
            MySqlType::Char(size) => (ColumnTypeFamily::String, format!("CHAR({size})", size = size)),
            MySqlType::VarChar(size) => (ColumnTypeFamily::String, format!("VARCHAR({size})", size = size)),
            MySqlType::Binary(size) => (ColumnTypeFamily::Binary, format!("BINARY({size})", size = size)),
            MySqlType::VarBinary(size) => (ColumnTypeFamily::Binary, format!("VARBINARY({size})", size = size)),
            MySqlType::TinyBlob => (ColumnTypeFamily::Binary, "TINYBLOB".into()),
            MySqlType::Blob => (ColumnTypeFamily::Binary, "BLOB".into()),
            MySqlType::MediumBlob => (ColumnTypeFamily::Binary, "MEDIUMBLOB".into()),
            MySqlType::LongBlob => (ColumnTypeFamily::Binary, "LONGBLOB".into()),
            MySqlType::TinyText => (ColumnTypeFamily::String, "TINYTEXT".into()),
            MySqlType::Text => (ColumnTypeFamily::String, "TEXT".into()),
            MySqlType::MediumText => (ColumnTypeFamily::String, "MEDIUMTEXT".into()),
            MySqlType::LongText => (ColumnTypeFamily::String, "LONGTEXT".into()),
            MySqlType::Date => (ColumnTypeFamily::DateTime, "DATE".into()),
            MySqlType::Time(precision) => (ColumnTypeFamily::DateTime, format!("TIME{}", render(precision))),
            MySqlType::DateTime(precision) => (ColumnTypeFamily::DateTime, format!("DATETIME{}", render(precision))),
            MySqlType::Timestamp(precision) => (ColumnTypeFamily::DateTime, format!("TIMESTAMP{}", render(precision))),
            MySqlType::Year => (ColumnTypeFamily::Int, "YEAR".into()),
            MySqlType::Json => (ColumnTypeFamily::Json, "JSON".into()),
            MySqlType::UnsignedInt => (ColumnTypeFamily::Int, "INTEGER UNSIGNED".into()),
            MySqlType::UnsignedSmallInt => (ColumnTypeFamily::Int, "SMALLINT UNSIGNED".into()),
            MySqlType::UnsignedTinyInt => (ColumnTypeFamily::Int, "TINYINT UNSIGNED".into()),
            MySqlType::UnsignedMediumInt => (ColumnTypeFamily::Int, "MEDIUMINT UNSIGNED".into()),
            MySqlType::UnsignedBigInt => (ColumnTypeFamily::BigInt, "BIGINT UNSIGNED".into()),
        };

        sql::ColumnType {
            data_type: data_type.clone(),
            full_data_type: data_type,
            character_maximum_length: None,
            family,
            arity: match field.arity() {
                datamodel::FieldArity::Required => sql::ColumnArity::Required,
                datamodel::FieldArity::Optional => sql::ColumnArity::Nullable,
                datamodel::FieldArity::List => sql::ColumnArity::List,
            },
            native_type: Some(native_type_instance.serialized_native_type.clone()),
        }
    }

    fn enum_column_type(&self, field: &ScalarFieldWalker<'_>, _db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.arity());

        sql::ColumnType::pure(
            sql::ColumnTypeFamily::Enum(format!("{}_{}", field.model().db_name(), field.db_name())),
            arity,
        )
    }

    fn default_native_type_for_family(&self, family: sql::ColumnTypeFamily) -> Option<serde_json::Value> {
        let ty = match family {
            ColumnTypeFamily::Int => MySqlType::Int,
            ColumnTypeFamily::BigInt => MySqlType::BigInt,
            ColumnTypeFamily::Float => MySqlType::Decimal(Some((65, 30))),
            ColumnTypeFamily::Decimal => MySqlType::Decimal(Some((65, 30))),
            ColumnTypeFamily::Boolean => MySqlType::TinyInt,
            ColumnTypeFamily::String => MySqlType::VarChar(191),
            ColumnTypeFamily::DateTime => MySqlType::DateTime(Some(3)),
            ColumnTypeFamily::Binary => MySqlType::VarBinary(191),
            ColumnTypeFamily::Json => MySqlType::Json,
            ColumnTypeFamily::Uuid => MySqlType::VarChar(37),
            ColumnTypeFamily::Enum(_) => return None,
            ColumnTypeFamily::Unsupported(_) => return None,
        };

        Some(serde_json::to_value(ty).expect("MySqlType to JSON failed"))
    }
}
