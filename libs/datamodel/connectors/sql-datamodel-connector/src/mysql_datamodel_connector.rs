use datamodel::ast::WithName;
use datamodel_connector::error::ConnectorError;
use datamodel_connector::scalars::ScalarType;
use datamodel_connector::{Connector, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance};
use native_types::{MySqlType, NativeType, PostgresType};

const INT_TYPE_NAME: &str = "Int";
const SMALL_INT_TYPE_NAME: &str = "SmallInt";
const TINY_INT_TYPE_NAME: &str = "TinyInt";
const MEDIUM_INT_TYPE_NAME: &str = "MediumInt";
const BIG_INT_TYPE_NAME: &str = "BigInt";
// const DECIMAL_TYPE_NAME: &str = "Decimal";
// const NUMERIC_TYPE_NAME: &str = "Numeric";
const FLOAT_TYPE_NAME: &str = "Float";
const DOUBLE_TYPE_NAME: &str = "Double";
// const BIT_TYPE_NAME: &str = "Bit";
const CHAR_TYPE_NAME: &str = "Char";
const VAR_CHAR_TYPE_NAME: &str = "VarChar";
// const BINARY_TYPE_NAME: &str = "Binary";
// const VAR_BINARY_TYPE_NAME: &str = "VarBinary";
// const TINY_BLOB_TYPE_NAME: &str = "TinyBlob";
// const BLOB_TYPE_NAME: &str = "Blob";
// const MEDIUM_BLOB_TYPE_NAME: &str = "MediumBlob";
// const LONG_BLOB_TYPE_NAME: &str = "LongBlob";
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
        ];

        let int = NativeTypeConstructor::without_args(INT_TYPE_NAME, ScalarType::Int);
        let small_int = NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, ScalarType::Int);
        let tiny_int = NativeTypeConstructor::without_args(TINY_INT_TYPE_NAME, ScalarType::Int);
        let medium_int = NativeTypeConstructor::without_args(MEDIUM_INT_TYPE_NAME, ScalarType::Int);
        let big_int = NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, ScalarType::Int);
        // let decimal = NativeTypeConstructor::with_args(DECIMAL_TYPE_NAME, 2, ScalarType::Decimal);
        // let numeric = NativeTypeConstructor::with_args(NUMERIC_TYPE_NAME, 2, ScalarType::Decimal);
        let float = NativeTypeConstructor::without_args(FLOAT_TYPE_NAME, ScalarType::Float);
        let double = NativeTypeConstructor::without_args(DOUBLE_TYPE_NAME, ScalarType::Float);
        // let bit = NativeTypeConstructor::with_args(BIT_TYPE_NAME, 1, ScalarType::Bit);
        let char = NativeTypeConstructor::with_args(CHAR_TYPE_NAME, 1, ScalarType::String);
        let var_char = NativeTypeConstructor::with_args(VAR_CHAR_TYPE_NAME, 1, ScalarType::String);
        // let binary = NativeTypeConstructor::with_args(BINARY_TYPE_NAME, 1, ScalarType::Bytes);
        // let var_binary = NativeTypeConstructor::with_args(VAR_BINARY_TYPE_NAME, 1, ScalarType::Bytes);
        // let tiny_blob = NativeTypeConstructor::without_args(TINY_BLOB_TYPE_NAME, ScalarType::Bytes);
        // let blob = NativeTypeConstructor::without_args(BLOB_TYPE_NAME, ScalarType::Bytes);
        // let medium_blob = NativeTypeConstructor::without_args(MEDIUM_BLOB_TYPE_NAME, ScalarType::Bytes);
        // let long_blob = NativeTypeConstructor::without_args(LONG_BLOB_TYPE_NAME, ScalarType::Bytes);
        let tiny_text = NativeTypeConstructor::without_args(TINY_TEXT_TYPE_NAME, ScalarType::String);
        let text = NativeTypeConstructor::without_args(TEXT_TYPE_NAME, ScalarType::String);
        let medium_text = NativeTypeConstructor::without_args(MEDIUM_TEXT_TYPE_NAME, ScalarType::String);
        let long_text = NativeTypeConstructor::without_args(LONG_TEXT_TYPE_NAME, ScalarType::String);
        let date = NativeTypeConstructor::without_args(DATE_TYPE_NAME, ScalarType::DateTime);
        let time = NativeTypeConstructor::with_args(TIME_TYPE_NAME, 1, ScalarType::DateTime);
        let datetime_with_arg = NativeTypeConstructor::without_args(DATETIME_TYPE_NAME, ScalarType::DateTime);
        let datetime_without_arg = NativeTypeConstructor::with_args(DATETIME_TYPE_NAME, 1, ScalarType::DateTime);
        let timestamp_with_arg = NativeTypeConstructor::with_args(TIMESTAMP_TYPE_NAME, 1, ScalarType::DateTime);
        let timestamp_without_arg = NativeTypeConstructor::without_args(TIMESTAMP_TYPE_NAME, ScalarType::DateTime);
        let year = NativeTypeConstructor::without_args(YEAR_TYPE_NAME, ScalarType::Int);
        let json = NativeTypeConstructor::without_args(JSON_TYPE_NAME, ScalarType::Json);

        let constructors: Vec<NativeTypeConstructor> = vec![
            int,
            small_int,
            tiny_int,
            medium_int,
            big_int,
            float,
            double,
            char,
            var_char,
            tiny_text,
            text,
            medium_text,
            long_text,
            date,
            time,
            datetime_with_arg,
            datetime_without_arg,
            timestamp_with_arg,
            timestamp_without_arg,
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

    fn parse_native_type(
        &self,
        name: &str,
        args: Vec<u32>,
        scalar_type: ScalarType,
    ) -> Result<NativeTypeInstance, ConnectorError> {
        let constructor = self.find_native_type_constructor(name);
        let length = *args.len();
        let native_type = match name {
            INT_TYPE_NAME => MySqlType::Int,
            SMALL_INT_TYPE_NAME => MySqlType::SmallInt,
            TINY_INT_TYPE_NAME => MySqlType::TinyInt,
            MEDIUM_INT_TYPE_NAME => MySqlType::MediumInt,
            BIG_INT_TYPE_NAME => MySqlType::BigInt,
            FLOAT_TYPE_NAME => MySqlType::Float,
            DOUBLE_TYPE_NAME => MySqlType::Double,
            CHAR_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    MySqlType::Char(arg)
                }
            }
            VAR_CHAR_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    MySqlType::VarChar(arg)
                }
            }
            TINY_TEXT_TYPE_NAME => MySqlType::TinyText,
            TEXT_TYPE_NAME => MySqlType::Text,
            MEDIUM_TEXT_TYPE_NAME => MySqlType::MediumText,
            LONG_TEXT_TYPE_NAME => MySqlType::LongText,
            DATE_TYPE_NAME => MySqlType::Date,
            TIME_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    MySqlType::Time(arg)
                } else {
                    MySqlType::Time(None)
                }
            }
            DATETIME_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    MySqlType::DateTime(arg)
                } else {
                    MySqlType::DateTime(None)
                }
            }
            YEAR_TYPE_NAME => MySqlType::Year,
            JSON_TYPE_NAME => MySqlType::JSON,

            _ => unreachable!("This code is unreachable as the core must guarantee to just call with known names."),
        };

        let native_type_constructor = self.constructors.iter().find(|c| c.name == name)?;

        if native_type_constructor._number_of_args != length {
            return Err(ConnectorError::new_argument_count_mismatch_error(
                name,
                native_type_constructor._number_of_args,
                length,
            ));
        }

        // check for compatability with scalar type
        let compatable_prisma_scalar_type = native_type_constructor.prisma_type;
        if compatable_prisma_scalar_type != scalar_type {
            return Err(ConnectorError::new_incompatible_native_type_error(
                "MySql",
                scalar_type,
                compatable_prisma_scalar_type,
            ));
        }

        match constructor {
            Some(constructor) => Ok(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type)),
            _ => panic!("",),
        }
    }

    fn introspect_native_type(&self, native_type: Box<dyn NativeType>) -> Result<NativeTypeInstance, ConnectorError> {
        let native_type: MySqlType = serde_json::from_value(native_type.to_json()).unwrap();
        let (constructor_name, args) = match native_type {
            MySqlType::Int => (INT_TYPE_NAME, vec![]),
            MySqlType::SmallInt => (SMALL_INT_TYPE_NAME, vec![]),
            MySqlType::TinyInt => (TINY_INT_TYPE_NAME, vec![]),
            MySqlType::MediumInt => (MEDIUM_INT_TYPE_NAME, vec![]),
            MySqlType::BigInt => (BIG_INT_TYPE_NAME, vec![]),
            MySqlType::Float => (FLOAT_TYPE_NAME, vec![]),
            MySqlType::Double => (DOUBLE_TYPE_NAME, vec![]),
            MySqlType::Char(x) => (CHAR_TYPE_NAME, vec![x]),
            MySqlType::VarChar(x) => (VAR_CHAR_TYPE_NAME, vec![x]),
            MySqlType::TinyText => (TINY_TEXT_TYPE_NAME, vec![]),
            MySqlType::Text => (TEXT_TYPE_NAME, vec![]),
            MySqlType::MediumText => (MEDIUM_TEXT_TYPE_NAME, vec![]),
            MySqlType::LongText => (LONG_TEXT_TYPE_NAME, vec![]),
            MySqlType::Date => (DATE_TYPE_NAME, vec![]),
            MySqlType::Time(x) => (TIME_TYPE_NAME, vec![x]),
            MySqlType::DateTime(x) => (DATETIME_TYPE_NAME, vec![x]),
            MySqlType::Timestamp(x) => (TIMESTAMP_TYPE_NAME, vec![x]),
            MySqlType::Year => (YEAR_TYPE_NAME, vec![]),
            MySqlType::JSON => (JSON_TYPE_NAME, vec![]),

            _ => {
                return Err(ConnectorError::new_type_name_unknown_error(
                    native_type.clone(),
                    "MySql",
                ));
            }
        };

        let constructor = self.find_native_type_constructor(constructor_name);

        match constructor {
            Some(constructor) => Ok(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type)),
            _ => panic!("",),
        }
    }
}
