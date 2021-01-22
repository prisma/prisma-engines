use super::SqlSchemaCalculatorFlavour;
use crate::flavour::MssqlFlavour;
use datamodel::{
    walkers::{ModelWalker, ScalarFieldWalker},
    FieldArity, NativeTypeInstance, ScalarType,
};
use native_types::{MsSqlType, MsSqlTypeParameter, NativeType};
use sql_schema_describer::{ColumnArity, ColumnType, ColumnTypeFamily, ForeignKeyAction};

impl SqlSchemaCalculatorFlavour for MssqlFlavour {
    fn column_type_for_native_type(
        &self,
        field: &ScalarFieldWalker<'_>,
        _scalar_type: ScalarType,
        native_type_instance: &NativeTypeInstance,
    ) -> ColumnType {
        use MsSqlType::*;
        let mssql_type: MsSqlType = native_type_instance.deserialize_native_type();
        // todo should this go into the datamodel connector with all the other native type stuff
        // maybe under render native type?
        let (family, data_type) = match mssql_type {
            TinyInt => (ColumnTypeFamily::Int, "tinyint".to_string()),
            SmallInt => (ColumnTypeFamily::Int, "smallint".to_string()),
            Int => (ColumnTypeFamily::Int, "int".to_string()),
            BigInt => (ColumnTypeFamily::BigInt, "bigint".to_string()),
            Decimal(Some((p, s))) => (ColumnTypeFamily::Decimal, format!("decimal({p},{s})", p = p, s = s)),
            Decimal(None) => (ColumnTypeFamily::Decimal, "decimal".to_string()),
            Money => (ColumnTypeFamily::Decimal, "money".to_string()),
            SmallMoney => (ColumnTypeFamily::Decimal, "smallmoney".to_string()),
            Bit => (ColumnTypeFamily::Boolean, "bit".to_string()),
            Float(bits) => (
                ColumnTypeFamily::Float,
                format!("float{bits}", bits = format_u32_arg(bits)),
            ),
            Real => (ColumnTypeFamily::Float, "real".to_string()),
            Date => (ColumnTypeFamily::DateTime, "date".to_string()),
            Time => (ColumnTypeFamily::DateTime, "time".to_string()),
            DateTime => (ColumnTypeFamily::DateTime, "datetime".to_string()),
            DateTime2 => (ColumnTypeFamily::DateTime, "datetime2".to_string()),
            DateTimeOffset => (ColumnTypeFamily::DateTime, "datetimeoffset".to_string()),
            SmallDateTime => (ColumnTypeFamily::DateTime, "smalldatetime".to_string()),
            NChar(len) => (
                ColumnTypeFamily::String,
                format!("nchar{len}", len = format_u32_arg(len)),
            ),
            Char(len) => (
                ColumnTypeFamily::String,
                format!("char{len}", len = format_u32_arg(len)),
            ),
            VarChar(len) => (
                ColumnTypeFamily::String,
                format!("varchar{len}", len = format_type_param(len)),
            ),
            Text => (ColumnTypeFamily::String, "text".to_string()),
            NVarChar(len) => (
                ColumnTypeFamily::String,
                format!("nvarchar{len}", len = format_type_param(len)),
            ),
            NText => (ColumnTypeFamily::String, "ntext".to_string()),
            Binary(len) => (
                ColumnTypeFamily::Binary,
                format!("binary{len}", len = format_u32_arg(len)),
            ),
            VarBinary(len) => (
                ColumnTypeFamily::Binary,
                format!("varbinary{len}", len = format_type_param(len)),
            ),
            Image => (ColumnTypeFamily::Binary, "image".to_string()),
            Xml => (ColumnTypeFamily::String, "xml".to_string()),
            UniqueIdentifier => (ColumnTypeFamily::Uuid, "uniqueidentifier".to_string()),
        };

        fn format_u32_arg(arg: Option<u32>) -> String {
            match arg {
                None => "".to_string(),
                Some(x) => format!("({})", x),
            }
        }
        fn format_type_param(arg: Option<MsSqlTypeParameter>) -> String {
            match arg {
                None => "".to_string(),
                Some(MsSqlTypeParameter::Number(x)) => format!("({})", x),
                Some(MsSqlTypeParameter::Max) => "(max)".to_string(),
            }
        }

        ColumnType {
            full_data_type: data_type,
            family,
            arity: match field.arity() {
                FieldArity::Required => ColumnArity::Required,
                FieldArity::Optional => ColumnArity::Nullable,
                FieldArity::List => ColumnArity::List,
            },
            native_type: Some(mssql_type.to_json()),
        }
    }

    fn m2m_foreign_key_action(&self, model_a: &ModelWalker<'_>, model_b: &ModelWalker<'_>) -> ForeignKeyAction {
        // MSSQL will crash when creating a cyclic cascade
        if model_a.name() == model_b.name() {
            ForeignKeyAction::NoAction
        } else {
            ForeignKeyAction::Cascade
        }
    }

    fn single_field_index_name(&self, model_name: &str, field_name: &str) -> String {
        format!("{}_{}_unique", model_name, field_name)
    }

    fn default_native_type_for_family(&self, family: ColumnTypeFamily) -> Option<serde_json::Value> {
        let ty = match family {
            ColumnTypeFamily::Int => MsSqlType::Int,
            ColumnTypeFamily::BigInt => MsSqlType::BigInt,
            ColumnTypeFamily::Float => MsSqlType::Decimal(Some((65, 30))),
            ColumnTypeFamily::Decimal => MsSqlType::Decimal(Some((65, 30))),
            ColumnTypeFamily::Boolean => MsSqlType::Bit,
            ColumnTypeFamily::String => MsSqlType::NVarChar(Some(MsSqlTypeParameter::Number(1000))),
            ColumnTypeFamily::DateTime => MsSqlType::DateTime2,
            ColumnTypeFamily::Binary => MsSqlType::VarBinary(Some(MsSqlTypeParameter::Max)),
            ColumnTypeFamily::Json => MsSqlType::NVarChar(Some(MsSqlTypeParameter::Number(1000))),
            ColumnTypeFamily::Uuid => MsSqlType::UniqueIdentifier,
            ColumnTypeFamily::Enum(_) => return None,
            ColumnTypeFamily::Unsupported(_) => return None,
        };

        Some(serde_json::to_value(ty).expect("MySqlType to JSON failed"))
    }
}
