use datamodel_connector::error::ConnectorError;
use datamodel_connector::{
    scalars::ScalarType, Connector, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance,
};
use native_types::{NativeType, PostgresType};

const SMALL_INT_TYPE_NAME: &str = "SmallInt";
const INTEGER_TYPE_NAME: &str = "Integer";
const BIG_INT_TYPE_NAME: &str = "BigInt";
// const DECIMAL_TYPE_NAME: &str = "Decimal";
// const NUMERIC_TYPE_NAME: &str = "Numeric";
const REAL_TYPE_NAME: &str = "Real";
const DOUBLE_PRECISION_TYPE_NAME: &str = "DoublePrecision";
const SMALL_SERIAL_TYPE_NAME: &str = "SmallSerial";
const SERIAL_TYPE_NAME: &str = "Serial";
const BIG_SERIAL_TYPE_NAME: &str = "BigSerial";
const VARCHAR_TYPE_NAME: &str = "VarChar";
const CHAR_TYPE_NAME: &str = "Char";
const TEXT_TYPE_NAME: &str = "Text";
// const BYTE_A_TYPE_NAME: &str = "ByteA";
const TIMESTAMP_TYPE_NAME: &str = "Timestamp";
const TIMESTAMP_WITH_TIMEZONE_TYPE_NAME: &str = "TimestampWithTimeZone";
const DATE_TYPE_NAME: &str = "Date";
const TIME_TYPE_NAME: &str = "Time";
const TIME_WITH_TIMEZONE_TYPE_NAME: &str = "TimeWithTimezone";
// const INTERVAL_TYPE_NAME: &str = "Interval";
const BOOLEAN_TYPE_NAME: &str = "Boolean";
const BIT_TYPE_NAME: &str = "Bit";
const VAR_BIT_TYPE_NAME: &str = "VarBit";
const UUID_TYPE_NAME: &str = "UUID";
// const XML_TYPE_NAME: &str = "XML";
const JSON_TYPE_NAME: &str = "Json";
const JSON_B_TYPE_NAME: &str = "JsonB";

pub struct PostgresDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl PostgresDatamodelConnector {
    pub fn new() -> PostgresDatamodelConnector {
        let capabilities = vec![
            ConnectorCapability::ScalarLists,
            ConnectorCapability::Enums,
            ConnectorCapability::Json,
            ConnectorCapability::AutoIncrementMultipleAllowed,
            ConnectorCapability::AutoIncrementAllowedOnNonId,
            ConnectorCapability::AutoIncrementNonIndexedAllowed,
        ];

        let small_int = NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, ScalarType::Int);
        let integer = NativeTypeConstructor::without_args(INTEGER_TYPE_NAME, ScalarType::Int);
        let big_int = NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, ScalarType::Int);
        // let decimal = NativeTypeConstructor::with_args(DECIMAL_TYPE_NAME, 2, ScalarType::Decimal);
        // let numeric = NativeTypeConstructor::with_args(NUMERIC_TYPE_NAME, 2, ScalarType::Decimal);
        let real = NativeTypeConstructor::without_args(REAL_TYPE_NAME, ScalarType::Float);
        let double_precision = NativeTypeConstructor::without_args(DOUBLE_PRECISION_TYPE_NAME, ScalarType::Float);
        let small_serial = NativeTypeConstructor::without_args(SMALL_SERIAL_TYPE_NAME, ScalarType::Int);
        let big_serial = NativeTypeConstructor::without_args(BIG_SERIAL_TYPE_NAME, ScalarType::Int);
        let varchar = NativeTypeConstructor::with_args(VARCHAR_TYPE_NAME, 1, ScalarType::String);
        let char = NativeTypeConstructor::with_args(CHAR_TYPE_NAME, 1, ScalarType::String);
        let text = NativeTypeConstructor::without_args(TEXT_TYPE_NAME, ScalarType::String);
        // let byte_a = NativeTypeConstructor::without_args(BYTE_A_TYPE_NAME, ScalarType::Bytes);
        let timestamp = NativeTypeConstructor::with_args(TIMESTAMP_TYPE_NAME, 1, ScalarType::DateTime);
        let timestamp_with_timezone =
            NativeTypeConstructor::with_args(TIMESTAMP_WITH_TIMEZONE_TYPE_NAME, 1, ScalarType::DateTime);
        let date = NativeTypeConstructor::without_args(DATE_TYPE_NAME, ScalarType::DateTime);
        let time = NativeTypeConstructor::with_args(TIME_TYPE_NAME, 1, ScalarType::DateTime);
        let time_with_timezone =
            NativeTypeConstructor::with_args(TIME_WITH_TIMEZONE_TYPE_NAME, 1, ScalarType::DateTime);
        // let interval = NativeTypeConstructor::with_args(INTERVAL_TYPE_NAME, 1, ScalarType::Interval);
        let boolean = NativeTypeConstructor::without_args(BOOLEAN_TYPE_NAME, ScalarType::Boolean);
        let bit = NativeTypeConstructor::with_args(BIT_TYPE_NAME, 1, ScalarType::String);
        let varbit = NativeTypeConstructor::with_args(VAR_BIT_TYPE_NAME, 1, ScalarType::String);
        let uuid = NativeTypeConstructor::without_args(UUID_TYPE_NAME, ScalarType::String);
        // let xml = NativeTypeConstructor::without_args(XML_TYPE_NAME, ScalarType::XML);
        let json = NativeTypeConstructor::without_args(JSON_TYPE_NAME, ScalarType::Json);
        let json_b = NativeTypeConstructor::without_args(JSON_B_TYPE_NAME, ScalarType::Json);

        let constructors = vec![
            small_int,
            integer,
            big_int,
            real,
            double_precision,
            small_serial,
            big_serial,
            varchar,
            char,
            text,
            timestamp,
            timestamp_with_timezone,
            date,
            time,
            time_with_timezone,
            boolean,
            bit,
            varbit,
            uuid,
            json,
            json_b,
        ];

        PostgresDatamodelConnector {
            capabilities,
            constructors,
        }
    }
}

impl Connector for PostgresDatamodelConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<u32>, scalar_type: ScalarType) -> Result<NativeTypeInstance, ConnectorError> {
        let constructor = self.find_native_type_constructor(name);
        let native_type = match name {
            SMALL_INT_TYPE_NAME => PostgresType::SmallInt,
            INTEGER_TYPE_NAME => PostgresType::Integer,
            BIG_INT_TYPE_NAME => PostgresType::BigInt,
            REAL_TYPE_NAME => PostgresType::Real,
            DOUBLE_PRECISION_TYPE_NAME => PostgresType::DoublePrecision,
            SMALL_SERIAL_TYPE_NAME => PostgresType::SmallSerial,
            SERIAL_TYPE_NAME => PostgresType::Serial,
            BIG_SERIAL_TYPE_NAME => PostgresType::BigSerial,
            VARCHAR_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    PostgresType::VarChar(arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        VARCHAR_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            }
            CHAR_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    PostgresType::Char(arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        CHAR_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            },
            TEXT_TYPE_NAME => PostgresType::Text,
            TIMESTAMP_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    PostgresType::Timestamp(arg as u8)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        TIMESTAMP_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            },
            TIMESTAMP_WITH_TIMEZONE_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    PostgresType::TimestampWithTimeZone(arg as u8)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        TIMESTAMP_WITH_TIMEZONE_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            }
            DATE_TYPE_NAME => PostgresType::Date,
            TIME_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    PostgresType::Time(arg as u8)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        TIME_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            },
            TIME_WITH_TIMEZONE_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    PostgresType::TimeWithTimeZone(arg as u8)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        TIME_WITH_TIMEZONE_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            },
            BOOLEAN_TYPE_NAME => PostgresType::Boolean,
            BIT_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    PostgresType::Bit(arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        BIT_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            },
            VAR_BIT_TYPE_NAME => {
                if let Some(arg) = *args.first() {
                    PostgresType::VarBit(arg)
                } else {
                    return Err(ConnectorError::new_argument_count_mismatch_error(
                        VAR_BIT_TYPE_NAME,
                        1,
                        0,
                    ));
                }
            },
            UUID_TYPE_NAME => PostgresType::UUID,
            JSON_TYPE_NAME => PostgresType::JSON,
            JSON_B_TYPE_NAME => PostgresType::JSONB,
            _ => unreachable!("This code is unreachable as the core must guarantee to just call with known names."),
        };

        // check for compatability with scalar type
        let compatable_prisma_scalar_type = self.constructors.iter().find(|c|c.name == name)?.prisma_type;
        if compatable_prisma_scalar_type != scalar_type {
            return Err(ConnectorError::new_incompatible_native_type_error("Postgres", scalar_type, compatable_prisma_scalar_type));
        }

        match constructor {
            Some(constructor) => Ok(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type)),
            _ => panic!(""),
        }
    }

    fn introspect_native_type(&self, native_type: Box<dyn NativeType>) -> Result<NativeTypeInstance, ConnectorError> {
        let native_type: PostgresType = serde_json::from_value(native_type.to_json()).unwrap();
        let (constructor_name, args) = match native_type {
            PostgresType::SmallInt => (SMALL_INT_TYPE_NAME, vec![]),
            PostgresType::Integer => (INTEGER_TYPE_NAME, vec![]),
            PostgresType::BigInt => (BIG_INT_TYPE_NAME, vec![]),
            PostgresType::Real => (REAL_TYPE_NAME, vec![]),
            PostgresType::DoublePrecision => (DOUBLE_PRECISION_TYPE_NAME, vec![]),
            PostgresType::SmallSerial => (SMALL_SERIAL_TYPE_NAME, vec![]),
            PostgresType::Serial => (SMALL_SERIAL_TYPE_NAME, vec![]),
            PostgresType::BigSerial => (BIG_SERIAL_TYPE_NAME, vec![]),
            PostgresType::VarChar(x) => (VARCHAR_TYPE_NAME, vec![x]),
            PostgresType::Char(x) => (CHAR_TYPE_NAME, vec![x]),
            PostgresType::Text => (TEXT_TYPE_NAME, vec![]),
            PostgresType::Timestamp(x) => (TIMESTAMP_TYPE_NAME, vec![x]),
            PostgresType::TimestampWithTimeZone(x) => (TIMESTAMP_WITH_TIMEZONE_TYPE_NAME, vec![x]),
            PostgresType::Date => (DATE_TYPE_NAME, vec![]),
            PostgresType::Time(x) => (TIME_TYPE_NAME, vec![x]),
            PostgresType::TimeWithTimeZone(x) => (TIME_WITH_TIMEZONE_TYPE_NAME, vec![x]),
            PostgresType::Boolean => (BOOLEAN_TYPE_NAME, vec![]),
            PostgresType::Bit(x) => (BIT_TYPE_NAME, vec![x]),
            PostgresType::VarBit(x) => (VAR_BIT_TYPE_NAME, vec![x]),
            PostgresType::UUID => (UUID_TYPE_NAME, vec![]),
            PostgresType::JSON => (JSON_TYPE_NAME, vec![]),
            PostgresType::JSONB => (JSON_B_TYPE_NAME, vec![]),

            _ => {
                return Err(ConnectorError::new_type_name_unknown_error(
                    native_type.clone(),
                    "Postgres",
                ));
            }
        };

        let constructor = self.find_native_type_constructor(constructor_name);

        match constructor {
            Some(constructor) => Ok(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type)),
            _ => panic!(""),
        }
    }
}
