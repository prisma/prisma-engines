use datamodel_connector::connector_error::{ConnectorError, ErrorKind};
use datamodel_connector::helper::parse_u32_arguments;
use datamodel_connector::{Connector, ConnectorCapability};
use dml::field::{Field, FieldType};
use dml::model::Model;
use dml::native_type_constructor::NativeTypeConstructor;
use dml::native_type_instance::NativeTypeInstance;
use dml::scalars::ScalarType;
use native_types::{MsSqlTypeParameter, MssqlType};
use once_cell::sync::Lazy;
use MsSqlTypeParameter::*;
use MssqlType::*;

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
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn validate_field(&self, field: &Field) -> Result<(), ConnectorError> {
        match field.field_type() {
            FieldType::NativeType(_, native_type) => {
                let r#type: MssqlType = native_type.deserialize_native_type();

                match r#type {
                    Decimal(Some(params)) | Numeric(Some(params)) => match params {
                        (precision, scale) if scale > precision => Err(
                            ConnectorError::new_scale_larger_than_precision_error(&native_type.render(), "SQL Server"),
                        ),
                        (precision, _) if precision == 0 || precision > 38 => {
                            Err(ConnectorError::new_argument_m_out_of_range_error(
                                "Precision can range from 1 to 38.",
                                &native_type.render(),
                                "SQL Server",
                            ))
                        }
                        (_, scale) if scale > 38 => Err(ConnectorError::new_argument_m_out_of_range_error(
                            "Scale can range from 0 to 38.",
                            &native_type.render(),
                            "SQL Server",
                        )),
                        _ => Ok(()),
                    },
                    Float(Some(bits)) => match bits {
                        bits if bits == 0 || bits > 53 => Err(ConnectorError::new_argument_m_out_of_range_error(
                            "Bits can range from 1 to 53.",
                            &native_type.render(),
                            "SQL Server",
                        )),
                        _ => Ok(()),
                    },
                    typ if heap_allocated_types().contains(&typ) => {
                        if field.is_unique() {
                            Err(ConnectorError::new_incompatible_native_type_with_unique(
                                &native_type.render(),
                                "SQL Server",
                            ))
                        } else if field.is_id() {
                            Err(ConnectorError::new_incompatible_native_type_with_id(
                                &native_type.render(),
                                "SQL Server",
                            ))
                        } else {
                            Ok(())
                        }
                    }
                    NVarChar(Some(Number(p))) if p > 2000 => Err(ConnectorError::new_argument_m_out_of_range_error(
                        "Length can range from 1 to 2000. For larger sizes, use the `Max` variant.",
                        &native_type.render(),
                        "SQL Server",
                    )),
                    VarChar(Some(Number(p))) | VarBinary(Some(Number(p))) if p > 4000 => {
                        Err(ConnectorError::new_argument_m_out_of_range_error(
                            r#"Length can range from 1 to 4000. For larger sizes, use the `Max` variant."#,
                            &native_type.render(),
                            "SQL Server",
                        ))
                    }
                    NChar(Some(p)) if p > 2000 => Err(ConnectorError::new_argument_m_out_of_range_error(
                        "Length can range from 1 to 2000.",
                        &native_type.render(),
                        "SQL Server",
                    )),
                    Char(Some(p)) | Binary(Some(p)) if p > 4000 => {
                        Err(ConnectorError::new_argument_m_out_of_range_error(
                            "Length can range from 1 to 4000.",
                            &native_type.render(),
                            "SQL Server",
                        ))
                    }
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    fn validate_model(&self, _model: &Model) -> Result<(), ConnectorError> {
        Ok(())
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError> {
        let cloned_args = args.clone();
        let number_of_args = args.len();
        let native_type = match &name {
            &TINY_INT_TYPE_NAME => MssqlType::TinyInt,
            &SMALL_INT_TYPE_NAME => MssqlType::SmallInt,
            &INT_TYPE_NAME => MssqlType::Int,
            &BIG_INT_TYPE_NAME => MssqlType::BigInt,
            &DECIMAL_TYPE_NAME => match parse_u32_arguments(args)?.as_slice() {
                [precision, scale] => MssqlType::Decimal(Some((*precision as u8, *scale as u8))),
                [] => MssqlType::Decimal(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(DECIMAL_TYPE_NAME, 2, number_of_args)),
            },
            &NUMERIC_TYPE_NAME => match parse_u32_arguments(args)?.as_slice() {
                [precision, scale] => MssqlType::Numeric(Some((*precision as u8, *scale as u8))),
                [] => MssqlType::Numeric(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(DECIMAL_TYPE_NAME, 2, number_of_args)),
            },
            &MONEY_TYPE_NAME => MssqlType::Money,
            &SMALL_MONEY_TYPE_NAME => MssqlType::SmallMoney,
            &BIT_TYPE_NAME => MssqlType::Bit,
            &FLOAT_TYPE_NAME => match parse_u32_arguments(args)?.as_slice() {
                [x] => MssqlType::Float(Some((*x as u8))),
                [] => MssqlType::Float(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(DECIMAL_TYPE_NAME, 2, number_of_args)),
            },
            &REAL_TYPE_NAME => MssqlType::Real,
            &DATE_TYPE_NAME => MssqlType::Date,
            &TIME_TYPE_NAME => MssqlType::Time,
            &DATETIME_TYPE_NAME => MssqlType::DateTime,
            &DATETIME2_TYPE_NAME => MssqlType::DateTime2,
            &DATETIME_OFFSET_TYPE_NAME => MssqlType::DateTimeOffset,
            &SMALL_DATETIME_TYPE_NAME => MssqlType::SmallDateTime,
            &CHAR_TYPE_NAME => match parse_u32_arguments(args)?.as_slice() {
                [x] => MssqlType::Char(Some((*x as u16))),
                [] => MssqlType::Char(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(DECIMAL_TYPE_NAME, 2, number_of_args)),
            },
            &NCHAR_TYPE_NAME => match parse_u32_arguments(args)?.as_slice() {
                [x] => MssqlType::NChar(Some((*x as u16))),
                [] => MssqlType::NChar(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(DECIMAL_TYPE_NAME, 2, number_of_args)),
            },
            &VARCHAR_TYPE_NAME => MssqlType::VarChar(parse_mssql_type_parameter(args)),
            &TEXT_TYPE_NAME => MssqlType::Text,
            &NVARCHAR_TYPE_NAME => MssqlType::NVarChar(parse_mssql_type_parameter(args)),
            &NTEXT_TYPE_NAME => MssqlType::NText,
            &BINARY_TYPE_NAME => match parse_u32_arguments(args)?.as_slice() {
                [x] => MssqlType::Binary(Some((*x as u16))),
                [] => MssqlType::Binary(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(DECIMAL_TYPE_NAME, 2, number_of_args)),
            },
            &VAR_BINARY_TYPE_NAME => MssqlType::VarBinary(parse_mssql_type_parameter(args)),
            &IMAGE_TYPE_NAME => MssqlType::Image,
            &XML_TYPE_NAME => MssqlType::Xml,
            &UNIQUE_IDENTIFIER_TYPE_NAME => MssqlType::UniqueIdentifier,
            _ => panic!(),
        };

        Ok(NativeTypeInstance::new(name, cloned_args, &native_type))
    }

    fn introspect_native_type(&self, _native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        Err(ConnectorError::from_kind(
            ErrorKind::ConnectorNotSupportedForNativeTypes {
                connector_name: "mssql".to_string(),
            },
        ))
    }
}

static HEAP_ALLOCATED: Lazy<Vec<MssqlType>> = Lazy::new(|| {
    vec![
        MssqlType::Text,
        MssqlType::NText,
        MssqlType::Image,
        MssqlType::Xml,
        MssqlType::VarBinary(Some(Max)),
        MssqlType::VarChar(Some(Max)),
        MssqlType::NVarChar(Some(Max)),
    ]
});

/// A collection of types stored outside of the row to the heap, having
/// certain properties such as not allowed in keys or normal indices.
pub fn heap_allocated_types() -> &'static [MssqlType] {
    &*HEAP_ALLOCATED
}

fn parse_mssql_type_parameter(args: Vec<String>) -> Option<MsSqlTypeParameter> {
    if args.len() > 1 {
        unreachable!()
    };

    args.first().map(|arg| {
        let is_max = arg
            .split(",")
            .map(|s| s.trim())
            .any(|s| matches!(s, "max" | "MAX" | "Max" | "MaX" | "maX" | "mAx"));

        if is_max {
            MsSqlTypeParameter::Max
        } else {
            arg.parse().map(MsSqlTypeParameter::Number).unwrap()
        }
    })
}
