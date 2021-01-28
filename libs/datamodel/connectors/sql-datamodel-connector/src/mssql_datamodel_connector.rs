use datamodel_connector::connector_error::ConnectorError;
use datamodel_connector::helper::{arg_vec_from_opt, args_vec_from_opt, parse_one_opt_u32, parse_two_opt_u32};
use datamodel_connector::{Connector, ConnectorCapability};
use dml::field::{Field, FieldType};
use dml::model::{IndexType, Model};
use dml::native_type_constructor::NativeTypeConstructor;
use dml::native_type_instance::NativeTypeInstance;
use dml::scalars::ScalarType;
use native_types::{MsSqlType, MsSqlTypeParameter};
use once_cell::sync::Lazy;
use MsSqlType::*;
use MsSqlTypeParameter::*;

const TINY_INT_TYPE_NAME: &str = "TinyInt";
const SMALL_INT_TYPE_NAME: &str = "SmallInt";
const INT_TYPE_NAME: &str = "Int";
const BIG_INT_TYPE_NAME: &str = "BigInt";
const DECIMAL_TYPE_NAME: &str = "Decimal";
const NUMERIC_TYPE_NAME: &str = "Numeric";
const MONEY_TYPE_NAME: &str = "Money";
const SMALL_MONEY_TYPE_NAME: &str = "SmallMoney";
const BIT_TYPE_NAME: &str = "Bit";
const FLOAT_TYPE_NAME: &str = "Float";
const REAL_TYPE_NAME: &str = "Real";
const DATE_TYPE_NAME: &str = "Date";
const TIME_TYPE_NAME: &str = "Time";
const DATETIME_TYPE_NAME: &str = "DateTime";
const DATETIME2_TYPE_NAME: &str = "DateTime2";
const DATETIME_OFFSET_TYPE_NAME: &str = "DateTimeOffset";
const SMALL_DATETIME_TYPE_NAME: &str = "SmallDateTime";
const CHAR_TYPE_NAME: &str = "Char";
const NCHAR_TYPE_NAME: &str = "NChar";
const VARCHAR_TYPE_NAME: &str = "VarChar";
const TEXT_TYPE_NAME: &str = "Text";
const NVARCHAR_TYPE_NAME: &str = "NVarChar";
const NTEXT_TYPE_NAME: &str = "NText";
const BINARY_TYPE_NAME: &str = "Binary";
const VAR_BINARY_TYPE_NAME: &str = "VarBinary";
const IMAGE_TYPE_NAME: &str = "Image";
const XML_TYPE_NAME: &str = "Xml";
const UNIQUE_IDENTIFIER_TYPE_NAME: &str = "UniqueIdentifier";

pub struct MsSqlDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl MsSqlDatamodelConnector {
    pub fn new() -> MsSqlDatamodelConnector {
        let capabilities = vec![
            ConnectorCapability::AutoIncrementAllowedOnNonId,
            ConnectorCapability::AutoIncrementMultipleAllowed,
            ConnectorCapability::AutoIncrementNonIndexedAllowed,
            ConnectorCapability::CreateMany,
        ];

        let constructors: Vec<NativeTypeConstructor> = vec![
            NativeTypeConstructor::without_args(TINY_INT_TYPE_NAME, vec![ScalarType::Int]),
            NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, vec![ScalarType::Int]),
            NativeTypeConstructor::without_args(INT_TYPE_NAME, vec![ScalarType::Int]),
            NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, vec![ScalarType::BigInt]),
            NativeTypeConstructor::with_optional_args(DECIMAL_TYPE_NAME, 2, vec![ScalarType::Decimal]),
            NativeTypeConstructor::with_optional_args(NUMERIC_TYPE_NAME, 2, vec![ScalarType::Decimal]),
            NativeTypeConstructor::without_args(MONEY_TYPE_NAME, vec![ScalarType::Float]),
            NativeTypeConstructor::without_args(SMALL_MONEY_TYPE_NAME, vec![ScalarType::Float]),
            NativeTypeConstructor::without_args(BIT_TYPE_NAME, vec![ScalarType::Boolean, ScalarType::Int]),
            NativeTypeConstructor::with_optional_args(FLOAT_TYPE_NAME, 1, vec![ScalarType::Float]),
            NativeTypeConstructor::without_args(REAL_TYPE_NAME, vec![ScalarType::Float]),
            NativeTypeConstructor::without_args(DATE_TYPE_NAME, vec![ScalarType::DateTime]),
            NativeTypeConstructor::without_args(TIME_TYPE_NAME, vec![ScalarType::DateTime]),
            NativeTypeConstructor::without_args(DATETIME_TYPE_NAME, vec![ScalarType::DateTime]),
            NativeTypeConstructor::without_args(DATETIME2_TYPE_NAME, vec![ScalarType::DateTime]),
            NativeTypeConstructor::without_args(DATETIME_OFFSET_TYPE_NAME, vec![ScalarType::DateTime]),
            NativeTypeConstructor::without_args(SMALL_DATETIME_TYPE_NAME, vec![ScalarType::DateTime]),
            NativeTypeConstructor::with_optional_args(CHAR_TYPE_NAME, 1, vec![ScalarType::String]),
            NativeTypeConstructor::with_optional_args(NCHAR_TYPE_NAME, 1, vec![ScalarType::String]),
            NativeTypeConstructor::with_optional_args(VARCHAR_TYPE_NAME, 1, vec![ScalarType::String]),
            NativeTypeConstructor::without_args(TEXT_TYPE_NAME, vec![ScalarType::String]),
            NativeTypeConstructor::with_optional_args(NVARCHAR_TYPE_NAME, 1, vec![ScalarType::String]),
            NativeTypeConstructor::without_args(NTEXT_TYPE_NAME, vec![ScalarType::String]),
            NativeTypeConstructor::with_optional_args(BINARY_TYPE_NAME, 1, vec![ScalarType::Bytes]),
            NativeTypeConstructor::with_optional_args(VAR_BINARY_TYPE_NAME, 1, vec![ScalarType::Bytes]),
            NativeTypeConstructor::without_args(IMAGE_TYPE_NAME, vec![ScalarType::Bytes]),
            NativeTypeConstructor::without_args(XML_TYPE_NAME, vec![ScalarType::String]),
            NativeTypeConstructor::without_args(UNIQUE_IDENTIFIER_TYPE_NAME, vec![ScalarType::String]),
        ];

        MsSqlDatamodelConnector {
            capabilities,
            constructors,
        }
    }
}

impl Connector for MsSqlDatamodelConnector {
    fn name(&self) -> String {
        "SQL Server".to_string()
    }

    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn validate_field(&self, field: &Field) -> Result<(), ConnectorError> {
        match field.field_type() {
            FieldType::NativeType(_, native_type) => {
                let r#type: MsSqlType = native_type.deserialize_native_type();
                let error = self.native_instance_error(native_type);

                match r#type {
                    Decimal(Some((precision, scale))) if scale > precision => {
                        error.new_scale_larger_than_precision_error()
                    }
                    Decimal(Some((prec, _))) if prec == 0 || prec > 38 => {
                        error.new_argument_m_out_of_range_error("Precision can range from 1 to 38.")
                    }
                    Decimal(Some((_, scale))) if scale > 38 => {
                        error.new_argument_m_out_of_range_error("Scale can range from 0 to 38.")
                    }
                    Float(Some(bits)) if bits == 0 || bits > 53 => {
                        error.new_argument_m_out_of_range_error("Bits can range from 1 to 53.")
                    }
                    typ if heap_allocated_types().contains(&typ) && field.is_unique() => {
                        error.new_incompatible_native_type_with_unique()
                    }
                    typ if heap_allocated_types().contains(&typ) && field.is_id() => {
                        error.new_incompatible_native_type_with_id()
                    }
                    NVarChar(Some(Number(p))) if p > 2000 => error.new_argument_m_out_of_range_error(
                        "Length can range from 1 to 2000. For larger sizes, use the `Max` variant.",
                    ),
                    VarChar(Some(Number(p))) | VarBinary(Some(Number(p))) if p > 4000 => error
                        .new_argument_m_out_of_range_error(
                            r#"Length can range from 1 to 4000. For larger sizes, use the `Max` variant."#,
                        ),
                    NChar(Some(p)) if p > 2000 => {
                        error.new_argument_m_out_of_range_error("Length can range from 1 to 2000.")
                    }
                    Char(Some(p)) | Binary(Some(p)) if p > 4000 => {
                        error.new_argument_m_out_of_range_error("Length can range from 1 to 4000.")
                    }
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    fn validate_model(&self, model: &Model) -> Result<(), ConnectorError> {
        for index_definition in model.indices.iter() {
            let fields = index_definition.fields.iter().map(|f| model.find_field(f).unwrap());

            for field in fields {
                if let FieldType::NativeType(_, native_type) = field.field_type() {
                    let r#type: MsSqlType = native_type.deserialize_native_type();
                    let error = self.native_instance_error(native_type);

                    if heap_allocated_types().contains(&r#type) {
                        return if index_definition.tpe == IndexType::Unique {
                            error.new_incompatible_native_type_with_unique()
                        } else {
                            error.new_incompatible_native_type_with_index()
                        };
                    }
                }
            }
        }

        for id_field in model.id_fields.iter() {
            let field = model.find_field(id_field).unwrap();

            if let FieldType::NativeType(_, native_type) = field.field_type() {
                let r#type: MsSqlType = native_type.deserialize_native_type();

                if heap_allocated_types().contains(&r#type) {
                    return self
                        .native_instance_error(native_type)
                        .new_incompatible_native_type_with_id();
                }
            }
        }

        Ok(())
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError> {
        let cloned_args = args.clone();
        let native_type = match name {
            TINY_INT_TYPE_NAME => TinyInt,
            SMALL_INT_TYPE_NAME => SmallInt,
            INT_TYPE_NAME => Int,
            BIG_INT_TYPE_NAME => BigInt,
            DECIMAL_TYPE_NAME => Decimal(parse_two_opt_u32(args, DECIMAL_TYPE_NAME)?),
            MONEY_TYPE_NAME => Money,
            SMALL_MONEY_TYPE_NAME => SmallMoney,
            BIT_TYPE_NAME => Bit,
            FLOAT_TYPE_NAME => Float(parse_one_opt_u32(args, FLOAT_TYPE_NAME)?),
            REAL_TYPE_NAME => Real,
            DATE_TYPE_NAME => Date,
            TIME_TYPE_NAME => Time,
            DATETIME_TYPE_NAME => DateTime,
            DATETIME2_TYPE_NAME => DateTime2,
            DATETIME_OFFSET_TYPE_NAME => DateTimeOffset,
            SMALL_DATETIME_TYPE_NAME => SmallDateTime,
            CHAR_TYPE_NAME => Char(parse_one_opt_u32(args, CHAR_TYPE_NAME)?),
            NCHAR_TYPE_NAME => NChar(parse_one_opt_u32(args, NCHAR_TYPE_NAME)?),
            VARCHAR_TYPE_NAME => VarChar(parse_mssql_type_parameter(args)),
            TEXT_TYPE_NAME => Text,
            NVARCHAR_TYPE_NAME => NVarChar(parse_mssql_type_parameter(args)),
            NTEXT_TYPE_NAME => NText,
            BINARY_TYPE_NAME => Binary(parse_one_opt_u32(args, BINARY_TYPE_NAME)?),
            VAR_BINARY_TYPE_NAME => VarBinary(parse_mssql_type_parameter(args)),
            IMAGE_TYPE_NAME => Image,
            XML_TYPE_NAME => Xml,
            UNIQUE_IDENTIFIER_TYPE_NAME => UniqueIdentifier,
            _ => panic!(),
        };

        Ok(NativeTypeInstance::new(name, cloned_args, &native_type))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        let native_type: MsSqlType = serde_json::from_value(native_type).unwrap();

        let (constructor_name, args) = match native_type {
            TinyInt => (TINY_INT_TYPE_NAME, vec![]),
            SmallInt => (SMALL_INT_TYPE_NAME, vec![]),
            Int => (INT_TYPE_NAME, vec![]),
            BigInt => (BIG_INT_TYPE_NAME, vec![]),
            Decimal(x) => (DECIMAL_TYPE_NAME, args_vec_from_opt(x)),
            Money => (MONEY_TYPE_NAME, vec![]),
            SmallMoney => (SMALL_MONEY_TYPE_NAME, vec![]),
            Bit => (BIT_TYPE_NAME, vec![]),
            Float(x) => (FLOAT_TYPE_NAME, arg_vec_from_opt(x)),
            Real => (REAL_TYPE_NAME, vec![]),
            Date => (DATE_TYPE_NAME, vec![]),
            Time => (TIME_TYPE_NAME, vec![]),
            DateTime => (DATETIME_TYPE_NAME, vec![]),
            DateTime2 => (DATETIME2_TYPE_NAME, vec![]),
            DateTimeOffset => (DATETIME_OFFSET_TYPE_NAME, vec![]),
            SmallDateTime => (SMALL_DATETIME_TYPE_NAME, vec![]),
            Char(x) => (CHAR_TYPE_NAME, arg_vec_from_opt(x)),
            NChar(x) => (NCHAR_TYPE_NAME, arg_vec_from_opt(x)),
            VarChar(x) => (VARCHAR_TYPE_NAME, arg_vec_for_type_param(x)),
            Text => (TEXT_TYPE_NAME, vec![]),
            NVarChar(x) => (NVARCHAR_TYPE_NAME, arg_vec_for_type_param(x)),
            NText => (NTEXT_TYPE_NAME, vec![]),
            Binary(x) => (BINARY_TYPE_NAME, arg_vec_from_opt(x)),
            VarBinary(x) => (VAR_BINARY_TYPE_NAME, arg_vec_for_type_param(x)),
            Image => (IMAGE_TYPE_NAME, vec![]),
            Xml => (XML_TYPE_NAME, vec![]),
            UniqueIdentifier => (UNIQUE_IDENTIFIER_TYPE_NAME, vec![]),
        };

        if let Some(constructor) = self.find_native_type_constructor(constructor_name) {
            let stringified_args = args.iter().map(|arg| arg.to_string()).collect();
            Ok(NativeTypeInstance::new(
                constructor.name.as_str(),
                stringified_args,
                &native_type,
            ))
        } else {
            self.native_str_error(constructor_name).native_type_name_unknown()
        }
    }
}

impl Default for MsSqlDatamodelConnector {
    fn default() -> Self {
        Self::new()
    }
}

static HEAP_ALLOCATED: Lazy<Vec<MsSqlType>> = Lazy::new(|| {
    vec![
        Text,
        NText,
        Image,
        Xml,
        VarBinary(Some(Max)),
        VarChar(Some(Max)),
        NVarChar(Some(Max)),
    ]
});

/// A collection of types stored outside of the row to the heap, having
/// certain properties such as not allowed in keys or normal indices.
pub fn heap_allocated_types() -> &'static [MsSqlType] {
    &*HEAP_ALLOCATED
}

fn parse_mssql_type_parameter(args: Vec<String>) -> Option<MsSqlTypeParameter> {
    if args.len() > 1 {
        unreachable!()
    };

    args.first().map(|arg| {
        let is_max = arg
            .split(',')
            .map(|s| s.trim())
            .any(|s| matches!(s, "max" | "MAX" | "Max" | "MaX" | "maX" | "mAx"));

        if is_max {
            MsSqlTypeParameter::Max
        } else {
            arg.parse().map(MsSqlTypeParameter::Number).unwrap()
        }
    })
}

fn arg_vec_for_type_param(type_param: Option<MsSqlTypeParameter>) -> Vec<String> {
    match type_param {
        Some(MsSqlTypeParameter::Max) => vec!["Max".to_string()],
        Some(MsSqlTypeParameter::Number(l)) => vec![l.to_string()],
        None => vec![],
    }
}
