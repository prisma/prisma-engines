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

        let data_type = match mssql_type {
            TinyInt => "tinyint".to_string(),
            SmallInt => "smallint".to_string(),
            Int => "int".to_string(),
            BigInt => "bigint".to_string(),
            Decimal(Some((p, s))) => format!("decimal({p},{s})", p = p, s = s),
            Numeric(Some((p, s))) => format!("numeric({p},{s})", p = p, s = s),
            Decimal(None) => "decimal".to_string(),
            Numeric(None) => "numeric".to_string(),
            Money => "money".to_string(),
            SmallMoney => "smallmoney".to_string(),
            Bit => "bit".to_string(),
            Float(bits) => format!("float{bits}", bits = format_u32_arg(bits)),
            Real => "real".to_string(),
            Date => "date".to_string(),
            Time => "time".to_string(),
            DateTime => "datetime".to_string(),
            DateTime2 => "datetime2".to_string(),
            DateTimeOffset => "datetimeoffset".to_string(),
            SmallDateTime => "smalldatetime".to_string(),
            NChar(len) => format!("nchar{len}", len = format_u32_arg(len)),
            Char(len) => format!("char{len}", len = format_u32_arg(len)),
            VarChar(len) => format!("varchar{len}", len = format_type_param(len)),
            Text => "text".to_string(),
            NVarChar(len) => format!("nvarchar{len}", len = format_type_param(len)),
            NText => "ntext".to_string(),
            Binary(len) => format!("binary{len}", len = format_u32_arg(len)),
            VarBinary(len) => format!("varbinary{len}", len = format_type_param(len)),
            Image => "image".to_string(),
            Xml => "xml".to_string(),
            UniqueIdentifier => "uniqueidentifier".to_string(),
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
            data_type: data_type.clone(),
            full_data_type: data_type,
            character_maximum_length: None,
            family: ColumnTypeFamily::String,
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
}
