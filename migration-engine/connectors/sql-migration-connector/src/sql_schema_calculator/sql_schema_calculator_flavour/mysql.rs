use super::SqlSchemaCalculatorFlavour;
use crate::{flavour::MysqlFlavour, sql_schema_calculator::SqlSchemaCalculator};
use datamodel::{
    walkers::{walk_scalar_fields, ScalarFieldWalker},
    ScalarType,
};
use datamodel_connector::NativeTypeInstance;
use native_types::MySqlType;
use sql_schema_describer::{self as sql};

impl SqlSchemaCalculatorFlavour for MysqlFlavour {
    fn calculate_enums(&self, calculator: &SqlSchemaCalculator<'_>) -> Vec<sql::Enum> {
        // This is a lower bound for the size of the generated enums (we assume
        // each enum is used at least once).
        let mut enums = Vec::with_capacity(calculator.data_model.enums().len());

        let enum_fields = walk_scalar_fields(&calculator.data_model)
            .filter_map(|field| field.field_type().as_enum().map(|enum_ref| (field, enum_ref)));

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

        let data_type: String = match mysql_type {
            MySqlType::Int => "INTEGER".into(),
            MySqlType::SmallInt => "SMALLINT".into(),
            MySqlType::TinyInt => "TINYINT".into(),
            MySqlType::MediumInt => "MEDIUMINT".into(),
            MySqlType::BigInt => "BIGINT".into(),
            MySqlType::Decimal(precision, scale) => format!("DECIMAL({}, {})", precision, scale),
            MySqlType::Numeric(precision, scale) => format!("NUMERIC({}, {})", precision, scale),
            MySqlType::Float => "FLOAT".into(),
            MySqlType::Double => "DOUBLE".into(),
            MySqlType::Bit(size) => format!("BIT({size})", size = size),
            MySqlType::Char(size) => format!("CHAR({size})", size = size),
            MySqlType::VarChar(size) => format!("VARCHAR({size})", size = size),
            MySqlType::Binary(size) => format!("BINARY({size})", size = size),
            MySqlType::VarBinary(size) => format!("VARBINARY({size})", size = size),
            MySqlType::TinyBlob => "TINYBLOB".into(),
            MySqlType::Blob => "BLOB".into(),
            MySqlType::MediumBlob => "MEDIUMBLOB".into(),
            MySqlType::LongBlob => "LONGBLOB".into(),
            MySqlType::TinyText => "TINYTEXT".into(),
            MySqlType::Text => "TEXT".into(),
            MySqlType::MediumText => "MEDIUMTEXT".into(),
            MySqlType::LongText => "LONGTEXT".into(),
            MySqlType::Date => "DATE".into(),
            MySqlType::Time(Some(precision)) => format!("TIME({precision})", precision = precision),
            MySqlType::Time(None) => "TIME".into(),
            MySqlType::DateTime(Some(precision)) => format!("DATETIME({precision})", precision = precision),
            MySqlType::DateTime(None) => "DATETIME".into(),
            MySqlType::Timestamp(Some(precision)) => format!("TIMESTAMP({precision})", precision = precision),
            MySqlType::Timestamp(None) => "TIMESTAMP".into(),
            MySqlType::Year => "YEAR".into(),
            MySqlType::JSON => "JSON".into(),
            _ => todo!(),
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
        }
    }
}
