use datamodel_connector::error::{ConnectorError, ErrorKind};
use datamodel_connector::scalars::ScalarType;
use datamodel_connector::{Connector, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance};
use native_types::MySqlType;

const INT_TYPE_NAME: &str = "Int";
const UNSIGNED_INT_TYPE_NAME: &str = "UnsignedInt";
const SMALL_INT_TYPE_NAME: &str = "SmallInt";
const UNSIGNED_SMALL_INT_TYPE_NAME: &str = "UnsignedSmallInt";
const TINY_INT_TYPE_NAME: &str = "TinyInt";
const UNSIGNED_TINY_INT_TYPE_NAME: &str = "UnsignedTinyInt";
const MEDIUM_INT_TYPE_NAME: &str = "MediumInt";
const UNSIGNED_MEDIUM_INT_TYPE_NAME: &str = "UnsignedMediumInt";
const BIG_INT_TYPE_NAME: &str = "BigInt";
const UNSIGNED_BIG_INT_TYPE_NAME: &str = "UnsignedBigInt";
const DECIMAL_TYPE_NAME: &str = "Decimal";
const NUMERIC_TYPE_NAME: &str = "Numeric";
const FLOAT_TYPE_NAME: &str = "Float";
const DOUBLE_TYPE_NAME: &str = "Double";
const BIT_TYPE_NAME: &str = "Bit";
const CHAR_TYPE_NAME: &str = "Char";
const VAR_CHAR_TYPE_NAME: &str = "VarChar";
const BINARY_TYPE_NAME: &str = "Binary";
const VAR_BINARY_TYPE_NAME: &str = "VarBinary";
const TINY_BLOB_TYPE_NAME: &str = "TinyBlob";
const BLOB_TYPE_NAME: &str = "Blob";
const MEDIUM_BLOB_TYPE_NAME: &str = "MediumBlob";
const LONG_BLOB_TYPE_NAME: &str = "LongBlob";
const TINY_TEXT_TYPE_NAME: &str = "TinyText";
const TEXT_TYPE_NAME: &str = "Text";
const MEDIUM_TEXT_TYPE_NAME: &str = "MediumText";
const LONG_TEXT_TYPE_NAME: &str = "LongText";
const DATE_TYPE_NAME: &str = "Date";
const TIME_TYPE_NAME: &str = "Time";
const DATETIME_TYPE_NAME: &str = "Datetime";
const TIMESTAMP_TYPE_NAME: &str = "Timestamp";
const YEAR_TYPE_NAME: &str = "Year";
const JSON_TYPE_NAME: &str = "JSON";

pub struct MySqlDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl MySqlDatamodelConnector {
    pub fn new() -> MySqlDatamodelConnector {
        let capabilities = vec![
            ConnectorCapability::RelationsOverNonUniqueCriteria,
            ConnectorCapability::Enums,
            ConnectorCapability::Json,
            ConnectorCapability::MultipleIndexesWithSameName,
            ConnectorCapability::AutoIncrementAllowedOnNonId,
            ConnectorCapability::RelationsOverNullableField,
        ];

        let int = NativeTypeConstructor::without_args(INT_TYPE_NAME, ScalarType::Int);
        let unsigned_int = NativeTypeConstructor::without_args(UNSIGNED_INT_TYPE_NAME, ScalarType::Int);
        let small_int = NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, ScalarType::Int);
        let unsigned_small_int = NativeTypeConstructor::without_args(UNSIGNED_SMALL_INT_TYPE_NAME, ScalarType::Int);
        let tiny_int = NativeTypeConstructor::without_args(TINY_INT_TYPE_NAME, ScalarType::Int);
        let unsigned_tiny_int = NativeTypeConstructor::without_args(UNSIGNED_TINY_INT_TYPE_NAME, ScalarType::Int);
        let medium_int = NativeTypeConstructor::without_args(MEDIUM_INT_TYPE_NAME, ScalarType::Int);
        let unsigned_medium_int = NativeTypeConstructor::without_args(UNSIGNED_MEDIUM_INT_TYPE_NAME, ScalarType::Int);
        let big_int = NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, ScalarType::Int);
        let unsigned_big_int = NativeTypeConstructor::without_args(UNSIGNED_BIG_INT_TYPE_NAME, ScalarType::Int);
        let decimal = NativeTypeConstructor::with_args(DECIMAL_TYPE_NAME, 2, ScalarType::Decimal);
        let numeric = NativeTypeConstructor::with_args(NUMERIC_TYPE_NAME, 2, ScalarType::Decimal);
        let float = NativeTypeConstructor::without_args(FLOAT_TYPE_NAME, ScalarType::Float);
        let double = NativeTypeConstructor::without_args(DOUBLE_TYPE_NAME, ScalarType::Float);
        let bit = NativeTypeConstructor::with_args(BIT_TYPE_NAME, 1, ScalarType::Bytes);
        let char = NativeTypeConstructor::with_args(CHAR_TYPE_NAME, 1, ScalarType::String);
        let var_char = NativeTypeConstructor::with_args(VAR_CHAR_TYPE_NAME, 1, ScalarType::String);
        let binary = NativeTypeConstructor::with_args(BINARY_TYPE_NAME, 1, ScalarType::Bytes);
        let var_binary = NativeTypeConstructor::with_args(VAR_BINARY_TYPE_NAME, 1, ScalarType::Bytes);
        let tiny_blob = NativeTypeConstructor::without_args(TINY_BLOB_TYPE_NAME, ScalarType::Bytes);
        let blob = NativeTypeConstructor::without_args(BLOB_TYPE_NAME, ScalarType::Bytes);
        let medium_blob = NativeTypeConstructor::without_args(MEDIUM_BLOB_TYPE_NAME, ScalarType::Bytes);
        let long_blob = NativeTypeConstructor::without_args(LONG_BLOB_TYPE_NAME, ScalarType::Bytes);
        let tiny_text = NativeTypeConstructor::without_args(TINY_TEXT_TYPE_NAME, ScalarType::String);
        let text = NativeTypeConstructor::without_args(TEXT_TYPE_NAME, ScalarType::String);
        let medium_text = NativeTypeConstructor::without_args(MEDIUM_TEXT_TYPE_NAME, ScalarType::String);
        let long_text = NativeTypeConstructor::without_args(LONG_TEXT_TYPE_NAME, ScalarType::String);
        let date = NativeTypeConstructor::without_args(DATE_TYPE_NAME, ScalarType::DateTime);
        let time = NativeTypeConstructor::with_optional_args(TIME_TYPE_NAME, 1, ScalarType::DateTime);
        let datetime = NativeTypeConstructor::with_optional_args(DATETIME_TYPE_NAME, 1, ScalarType::DateTime);
        let timestamp = NativeTypeConstructor::with_optional_args(TIMESTAMP_TYPE_NAME, 1, ScalarType::DateTime);
        let year = NativeTypeConstructor::without_args(YEAR_TYPE_NAME, ScalarType::Int);
        let json = NativeTypeConstructor::without_args(JSON_TYPE_NAME, ScalarType::Json);

        let constructors: Vec<NativeTypeConstructor> = vec![
            int,
            unsigned_int,
            small_int,
            unsigned_small_int,
            tiny_int,
            unsigned_tiny_int,
            medium_int,
            unsigned_medium_int,
            big_int,
            unsigned_big_int,
            decimal,
            numeric,
            float,
            double,
            bit,
            char,
            var_char,
            binary,
            var_binary,
            tiny_blob,
            blob,
            medium_blob,
            long_blob,
            tiny_text,
            text,
            medium_text,
            long_text,
            date,
            time,
            datetime,
            timestamp,
            year,
            json,
        ];

        MySqlDatamodelConnector {
            capabilities,
            constructors,
        }
    }
}

impl Connector for MySqlDatamodelConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<u32>) -> Result<NativeTypeInstance, ConnectorError> {
        let constructor = self.find_native_type_constructor(name);
        let native_type = match name {
            INT_TYPE_NAME => MySqlType::Int,
            UNSIGNED_INT_TYPE_NAME => MySqlType::UnsignedInt,
            SMALL_INT_TYPE_NAME => MySqlType::SmallInt,
            UNSIGNED_SMALL_INT_TYPE_NAME => MySqlType::UnsignedSmallInt,
            TINY_INT_TYPE_NAME => MySqlType::TinyInt,
            UNSIGNED_TINY_INT_TYPE_NAME => MySqlType::UnsignedTinyInt,
            MEDIUM_INT_TYPE_NAME => MySqlType::MediumInt,
            UNSIGNED_MEDIUM_INT_TYPE_NAME => MySqlType::UnsignedMediumInt,
            BIG_INT_TYPE_NAME => MySqlType::BigInt,
            UNSIGNED_BIG_INT_TYPE_NAME => MySqlType::UnsignedBigInt,
            DECIMAL_TYPE_NAME => {
                if let (Some(first_arg), Some(second_arg)) = (args.get(0), args.get(1)) {
                    MySqlType::Decimal(*first_arg, *second_arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        DECIMAL_TYPE_NAME,
                        2,
                        args.len(),
                    ));
                }
            }
            NUMERIC_TYPE_NAME => {
                if let (Some(first_arg), Some(second_arg)) = (args.get(0), args.get(1)) {
                    MySqlType::Numeric(*first_arg, *second_arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        NUMERIC_TYPE_NAME,
                        2,
                        args.len(),
                    ));
                }
            }
            FLOAT_TYPE_NAME => MySqlType::Float,
            DOUBLE_TYPE_NAME => MySqlType::Double,
            BIT_TYPE_NAME => {
                if let Some(arg) = args.first() {
                    MySqlType::Bit(*arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(BIT_TYPE_NAME, 1, 0));
                }
            }
            CHAR_TYPE_NAME => {
                if let Some(arg) = args.first() {
                    MySqlType::Char(*arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(CHAR_TYPE_NAME, 1, 0));
                }
            }
            VAR_CHAR_TYPE_NAME => {
                if let Some(arg) = args.first() {
                    MySqlType::VarChar(*arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        VAR_CHAR_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            }
            BINARY_TYPE_NAME => {
                if let Some(arg) = args.first() {
                    MySqlType::Binary(*arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        BINARY_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            }
            VAR_BINARY_TYPE_NAME => {
                if let Some(arg) = args.first() {
                    MySqlType::VarBinary(*arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        VAR_BINARY_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            }
            TINY_BLOB_TYPE_NAME => MySqlType::TinyBlob,
            BLOB_TYPE_NAME => MySqlType::Blob,
            MEDIUM_BLOB_TYPE_NAME => MySqlType::MediumBlob,
            LONG_BLOB_TYPE_NAME => MySqlType::LongBlob,
            TINY_TEXT_TYPE_NAME => MySqlType::TinyText,
            TEXT_TYPE_NAME => MySqlType::Text,
            MEDIUM_TEXT_TYPE_NAME => MySqlType::MediumText,
            LONG_TEXT_TYPE_NAME => MySqlType::LongText,
            DATE_TYPE_NAME => MySqlType::Date,
            TIME_TYPE_NAME => MySqlType::Time(args.first().cloned()),
            DATETIME_TYPE_NAME => MySqlType::DateTime(args.first().cloned()),
            TIMESTAMP_TYPE_NAME => MySqlType::Timestamp(args.first().cloned()),
            YEAR_TYPE_NAME => MySqlType::Year,
            JSON_TYPE_NAME => MySqlType::JSON,
            x => unreachable!(format!(
                "This code is unreachable as the core must guarantee to just call with known names. {}",
                x
            )),
        };

        Ok(NativeTypeInstance::new(
            constructor.unwrap().name.as_str(),
            args,
            &native_type,
        ))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        let native_type: MySqlType = serde_json::from_value(native_type).unwrap();
        let (constructor_name, args) = match native_type {
            MySqlType::Int => (INT_TYPE_NAME, vec![]),
            MySqlType::UnsignedInt => (UNSIGNED_INT_TYPE_NAME, vec![]),
            MySqlType::SmallInt => (SMALL_INT_TYPE_NAME, vec![]),
            MySqlType::UnsignedSmallInt => (UNSIGNED_SMALL_INT_TYPE_NAME, vec![]),
            MySqlType::TinyInt => (TINY_INT_TYPE_NAME, vec![]),
            MySqlType::UnsignedTinyInt => (UNSIGNED_TINY_INT_TYPE_NAME, vec![]),
            MySqlType::MediumInt => (MEDIUM_INT_TYPE_NAME, vec![]),
            MySqlType::UnsignedMediumInt => (UNSIGNED_MEDIUM_INT_TYPE_NAME, vec![]),
            MySqlType::BigInt => (BIG_INT_TYPE_NAME, vec![]),
            MySqlType::UnsignedBigInt => (UNSIGNED_BIG_INT_TYPE_NAME, vec![]),
            MySqlType::Decimal(x, y) => (DECIMAL_TYPE_NAME, vec![x, y]),
            MySqlType::Numeric(x, y) => (NUMERIC_TYPE_NAME, vec![x, y]),
            MySqlType::Float => (FLOAT_TYPE_NAME, vec![]),
            MySqlType::Double => (DOUBLE_TYPE_NAME, vec![]),
            MySqlType::Bit(x) => (BIT_TYPE_NAME, vec![x]),
            MySqlType::Char(x) => (CHAR_TYPE_NAME, vec![x]),
            MySqlType::VarChar(x) => (VAR_CHAR_TYPE_NAME, vec![x]),
            MySqlType::Binary(x) => (BINARY_TYPE_NAME, vec![x]),
            MySqlType::VarBinary(x) => (VAR_BINARY_TYPE_NAME, vec![x]),
            MySqlType::TinyBlob => (TINY_BLOB_TYPE_NAME, vec![]),
            MySqlType::Blob => (BLOB_TYPE_NAME, vec![]),
            MySqlType::MediumBlob => (MEDIUM_BLOB_TYPE_NAME, vec![]),
            MySqlType::LongBlob => (LONG_BLOB_TYPE_NAME, vec![]),
            MySqlType::TinyText => (TINY_TEXT_TYPE_NAME, vec![]),
            MySqlType::Text => (TEXT_TYPE_NAME, vec![]),
            MySqlType::MediumText => (MEDIUM_TEXT_TYPE_NAME, vec![]),
            MySqlType::LongText => (LONG_TEXT_TYPE_NAME, vec![]),
            MySqlType::Date => (DATE_TYPE_NAME, vec![]),
            MySqlType::Time(x) => (TIME_TYPE_NAME, arg_vec_from_opt(x)),
            MySqlType::DateTime(x) => (DATETIME_TYPE_NAME, arg_vec_from_opt(x)),
            MySqlType::Timestamp(x) => (TIMESTAMP_TYPE_NAME, arg_vec_from_opt(x)),
            MySqlType::Year => (YEAR_TYPE_NAME, vec![]),
            MySqlType::JSON => (JSON_TYPE_NAME, vec![]),
        };

        fn arg_vec_from_opt(input: Option<u32>) -> Vec<u32> {
            match input {
                Some(arg) => vec![arg],
                None => vec![],
            }
        }

        if let Some(constructor) = self.find_native_type_constructor(constructor_name) {
            Ok(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type))
        } else {
            Err(ConnectorError::from_kind(ErrorKind::NativeTypeNameUnknown {
                native_type: constructor_name.parse().unwrap(),
                connector_name: "Mysql".parse().unwrap(),
            }))
        }
    }
}
