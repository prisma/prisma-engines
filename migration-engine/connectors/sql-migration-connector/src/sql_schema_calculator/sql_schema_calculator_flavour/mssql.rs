use super::SqlSchemaCalculatorFlavour;
use crate::flavour::MssqlFlavour;
use datamodel::{walkers::ScalarFieldWalker, FieldArity, ScalarType};
use datamodel_connector::NativeTypeInstance;
use native_types::MssqlType;
use sql_schema_describer::{ColumnArity, ColumnType, ColumnTypeFamily};

impl SqlSchemaCalculatorFlavour for MssqlFlavour {
    fn column_type_for_native_type(
        &self,
        field: &ScalarFieldWalker<'_>,
        _scalar_type: ScalarType,
        native_type_instance: &NativeTypeInstance,
    ) -> ColumnType {
        use MssqlType::*;
        let mssql_type: MssqlType = native_type_instance.deserialize_native_type();

        let data_type = match mssql_type {
            TinyInt => "tinyint".to_string(),
            SmallInt => "smallint".to_string(),
            Int => "int".to_string(),
            BigInt => "bigint".to_string(),
            Decimal(p, s) => format!("decimal({p},{s})", p = p, s = s),
            Numeric(p, s) => format!("numeric({p},{s})", p = p, s = s),
            Money => "money".to_string(),
            SmallMoney => "smallmoney".to_string(),
            Bit => "bit".to_string(),
            Float(bits) => format!("float({bits})", bits = bits),
            Real => "real".to_string(),
            Date => "date".to_string(),
            Time => "time".to_string(),
            Datetime => "datetime".to_string(),
            Datetime2 => "datetime2".to_string(),
            DatetimeOffset => "datetimeoffset".to_string(),
            SmallDatetime => "smalldatetime".to_string(),
            Char(len) => format!("char({len})", len = len),
            VarChar(len) => format!("varchar({len})", len = len),
            Text => "text".to_string(),
            NVarChar(len) => format!("nvarchar({len})", len = len),
            NText => "ntext".to_string(),
            Binary(len) => format!("binary({len})", len = len),
            VarBinary(len) => format!("varbinary({len})", len = len),
            Image => "image".to_string(),
            XML => "xml".to_string(),
        };

        ColumnType {
            data_type: data_type.clone(),
            full_data_type: data_type,
            character_maximum_length: None,
            family: ColumnTypeFamily::String,
            arity: match field.arity() {
                FieldArity::Required => ColumnArity::Required,
                FieldArity::Optional => ColumnArity::Nullable,
                FieldArity::List => ColumnArity::List,
            },
        }
    }
}
