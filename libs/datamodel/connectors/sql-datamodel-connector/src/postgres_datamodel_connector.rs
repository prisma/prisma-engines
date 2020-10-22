use datamodel_connector::connector_error::{ConnectorError, ErrorKind};
use datamodel_connector::helper::parse_u32_arguments;
use datamodel_connector::{Connector, ConnectorCapability};
use dml::default_value::DefaultValue;
use dml::field::{Field, FieldType};
use dml::model::Model;
use dml::native_type_constructor::NativeTypeConstructor;
use dml::native_type_instance::NativeTypeInstance;
use dml::scalars::ScalarType;
use native_types::PostgresType;

const SMALL_INT_TYPE_NAME: &str = "SmallInt";
const INTEGER_TYPE_NAME: &str = "Integer";
const BIG_INT_TYPE_NAME: &str = "BigInt";
const DECIMAL_TYPE_NAME: &str = "Decimal";
const NUMERIC_TYPE_NAME: &str = "Numeric";
const REAL_TYPE_NAME: &str = "Real";
const DOUBLE_PRECISION_TYPE_NAME: &str = "DoublePrecision";
const SMALL_SERIAL_TYPE_NAME: &str = "SmallSerial";
const SERIAL_TYPE_NAME: &str = "Serial";
const BIG_SERIAL_TYPE_NAME: &str = "BigSerial";
const VARCHAR_TYPE_NAME: &str = "VarChar";
const CHAR_TYPE_NAME: &str = "Char";
const TEXT_TYPE_NAME: &str = "Text";
const BYTE_A_TYPE_NAME: &str = "ByteA";
const TIMESTAMP_TYPE_NAME: &str = "Timestamp";
const TIMESTAMP_WITH_TIMEZONE_TYPE_NAME: &str = "TimestampWithTimeZone";
const DATE_TYPE_NAME: &str = "Date";
const TIME_TYPE_NAME: &str = "Time";
const TIME_WITH_TIMEZONE_TYPE_NAME: &str = "TimeWithTimeZone";
const INTERVAL_TYPE_NAME: &str = "Interval";
const BOOLEAN_TYPE_NAME: &str = "Boolean";
const BIT_TYPE_NAME: &str = "Bit";
const VAR_BIT_TYPE_NAME: &str = "VarBit";
const UUID_TYPE_NAME: &str = "Uuid";
const XML_TYPE_NAME: &str = "Xml";
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
            ConnectorCapability::InsensitiveFilters,
        ];

        let small_int = NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, vec![ScalarType::Int]);
        let integer = NativeTypeConstructor::without_args(INTEGER_TYPE_NAME, vec![ScalarType::Int]);
        let big_int = NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, vec![ScalarType::Int]);
        let decimal = NativeTypeConstructor::with_args(DECIMAL_TYPE_NAME, 2, vec![ScalarType::Decimal]);
        let numeric = NativeTypeConstructor::with_args(NUMERIC_TYPE_NAME, 2, vec![ScalarType::Decimal]);
        let real = NativeTypeConstructor::without_args(REAL_TYPE_NAME, vec![ScalarType::Float]);
        let double_precision = NativeTypeConstructor::without_args(DOUBLE_PRECISION_TYPE_NAME, vec![ScalarType::Float]);
        let small_serial = NativeTypeConstructor::without_args(SMALL_SERIAL_TYPE_NAME, vec![ScalarType::Int]);
        let serial = NativeTypeConstructor::without_args(SERIAL_TYPE_NAME, vec![ScalarType::Int]);
        let big_serial = NativeTypeConstructor::without_args(BIG_SERIAL_TYPE_NAME, vec![ScalarType::Int]);
        let varchar = NativeTypeConstructor::with_args(VARCHAR_TYPE_NAME, 1, vec![ScalarType::String]);
        let char = NativeTypeConstructor::with_args(CHAR_TYPE_NAME, 1, vec![ScalarType::String]);
        let text = NativeTypeConstructor::without_args(TEXT_TYPE_NAME, vec![ScalarType::String]);
        let byte_a = NativeTypeConstructor::without_args(BYTE_A_TYPE_NAME, vec![ScalarType::Bytes]);
        let timestamp = NativeTypeConstructor::with_args(TIMESTAMP_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let timestamp_with_timezone =
            NativeTypeConstructor::with_args(TIMESTAMP_WITH_TIMEZONE_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let date = NativeTypeConstructor::without_args(DATE_TYPE_NAME, vec![ScalarType::DateTime]);
        let time = NativeTypeConstructor::with_args(TIME_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let time_with_timezone =
            NativeTypeConstructor::with_args(TIME_WITH_TIMEZONE_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let interval = NativeTypeConstructor::with_args(INTERVAL_TYPE_NAME, 1, vec![ScalarType::Duration]);
        let boolean = NativeTypeConstructor::without_args(BOOLEAN_TYPE_NAME, vec![ScalarType::Boolean]);
        let bit = NativeTypeConstructor::with_args(BIT_TYPE_NAME, 1, vec![ScalarType::String]);
        let varbit = NativeTypeConstructor::with_args(VAR_BIT_TYPE_NAME, 1, vec![ScalarType::String]);
        let uuid = NativeTypeConstructor::without_args(UUID_TYPE_NAME, vec![ScalarType::String]);
        let xml = NativeTypeConstructor::without_args(XML_TYPE_NAME, vec![ScalarType::Xml]);
        let json = NativeTypeConstructor::without_args(JSON_TYPE_NAME, vec![ScalarType::Json]);
        let json_b = NativeTypeConstructor::without_args(JSON_B_TYPE_NAME, vec![ScalarType::Json]);

        let constructors = vec![
            small_int,
            integer,
            big_int,
            decimal,
            numeric,
            real,
            double_precision,
            small_serial,
            serial,
            big_serial,
            varchar,
            char,
            text,
            byte_a,
            timestamp,
            timestamp_with_timezone,
            date,
            time,
            time_with_timezone,
            interval,
            boolean,
            bit,
            varbit,
            uuid,
            xml,
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

    fn validate_field(&self, field: &Field) -> Result<(), ConnectorError> {
        if let FieldType::NativeType(_scalar_type, native_type) = field.field_type() {
            let native_type_name = native_type.name.as_str();
            if matches!(native_type_name, DECIMAL_TYPE_NAME | NUMERIC_TYPE_NAME) {
                match native_type.args.as_slice() {
                    [precision, scale] if scale > precision => {
                        return Err(ConnectorError::new_scale_larger_than_precision_error(
                            native_type_name,
                            "Postgres",
                        ));
                    }
                    _ => {}
                }
            }
            if matches!(native_type_name, BIT_TYPE_NAME | VAR_BIT_TYPE_NAME) {
                match native_type.args.as_slice() {
                    [length] if length == &0 => {
                        return Err(ConnectorError::new_argument_m_out_of_range_error(
                            "M must be a positive integer.",
                            native_type_name,
                            "MySQL",
                        ))
                    }
                    _ => {}
                }
            }
            if matches!(
                native_type_name,
                SMALL_SERIAL_TYPE_NAME | SERIAL_TYPE_NAME | BIG_SERIAL_TYPE_NAME
            ) {
                if let Some(DefaultValue::Single(_)) = field.default_value() {
                    return Err(
                        ConnectorError::new_incompatible_sequential_type_with_static_default_value_error(
                            native_type_name,
                            "Postgres",
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    fn validate_model(&self, _model: &Model) -> Result<(), ConnectorError> {
        Ok(())
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError> {
        let parsed_args = parse_u32_arguments(args)?;

        let constructor = self.find_native_type_constructor(name);
        let native_type = match name {
            SMALL_INT_TYPE_NAME => PostgresType::SmallInt,
            INTEGER_TYPE_NAME => PostgresType::Integer,
            BIG_INT_TYPE_NAME => PostgresType::BigInt,
            DECIMAL_TYPE_NAME => match parsed_args.as_slice() {
                [scale, precision] => PostgresType::Decimal(*scale, *precision),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(DECIMAL_TYPE_NAME, 2, parsed_args.len())),
            },
            NUMERIC_TYPE_NAME => match parsed_args.as_slice() {
                [scale, precision] => PostgresType::Numeric(*scale, *precision),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(NUMERIC_TYPE_NAME, 2, parsed_args.len())),
            },
            REAL_TYPE_NAME => PostgresType::Real,
            DOUBLE_PRECISION_TYPE_NAME => PostgresType::DoublePrecision,
            SMALL_SERIAL_TYPE_NAME => PostgresType::SmallSerial,
            SERIAL_TYPE_NAME => PostgresType::Serial,
            BIG_SERIAL_TYPE_NAME => PostgresType::BigSerial,
            VARCHAR_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::VarChar(*arg),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(VARCHAR_TYPE_NAME, 1, parsed_args.len())),
            },
            CHAR_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::Char(*arg),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(CHAR_TYPE_NAME, 1, parsed_args.len())),
            },
            TEXT_TYPE_NAME => PostgresType::Text,
            BYTE_A_TYPE_NAME => PostgresType::ByteA,
            TIMESTAMP_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::Timestamp(Option::Some(*arg)),
                [] => PostgresType::Timestamp(None),
                _ => {
                    return Err(self.wrap_in_optional_argument_count_mismatch_error(
                        TIMESTAMP_TYPE_NAME,
                        1,
                        parsed_args.len(),
                    ))
                }
            },
            TIMESTAMP_WITH_TIMEZONE_TYPE_NAME => PostgresType::TimestampWithTimeZone(parsed_args.first().cloned()),
            INTERVAL_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::Interval(Option::Some(*arg)),
                [] => PostgresType::Interval(None),
                _ => {
                    return Err(self.wrap_in_optional_argument_count_mismatch_error(
                        INTERVAL_TYPE_NAME,
                        1,
                        parsed_args.len(),
                    ))
                }
            },
            DATE_TYPE_NAME => PostgresType::Date,
            TIME_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::Time(Option::Some(*arg)),
                [] => PostgresType::Time(None),
                _ => {
                    return Err(self.wrap_in_optional_argument_count_mismatch_error(
                        TIME_TYPE_NAME,
                        1,
                        parsed_args.len(),
                    ))
                }
            },
            TIME_WITH_TIMEZONE_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::TimeWithTimeZone(Option::Some(*arg)),
                [] => PostgresType::TimeWithTimeZone(None),
                _ => {
                    return Err(self.wrap_in_optional_argument_count_mismatch_error(
                        TIME_WITH_TIMEZONE_TYPE_NAME,
                        1,
                        parsed_args.len(),
                    ))
                }
            },
            BOOLEAN_TYPE_NAME => PostgresType::Boolean,
            BIT_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::Bit(*arg),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(BIT_TYPE_NAME, 1, parsed_args.len())),
            },
            VAR_BIT_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::VarBit(*arg),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(VAR_BIT_TYPE_NAME, 1, parsed_args.len())),
            },
            UUID_TYPE_NAME => PostgresType::UUID,
            XML_TYPE_NAME => PostgresType::Xml,
            JSON_TYPE_NAME => PostgresType::JSON,
            JSON_B_TYPE_NAME => PostgresType::JSONB,
            _ => unreachable!("This code is unreachable as the core must guarantee to just call with known names."),
        };

        Ok(NativeTypeInstance::new(
            constructor.unwrap().name.as_str(),
            parsed_args,
            &native_type,
        ))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        let native_type: PostgresType = serde_json::from_value(native_type).unwrap();
        let (constructor_name, args) = match native_type {
            PostgresType::SmallInt => (SMALL_INT_TYPE_NAME, vec![]),
            PostgresType::Integer => (INTEGER_TYPE_NAME, vec![]),
            PostgresType::BigInt => (BIG_INT_TYPE_NAME, vec![]),
            PostgresType::Decimal(x, y) => (DECIMAL_TYPE_NAME, vec![x, y]),
            PostgresType::Numeric(x, y) => (NUMERIC_TYPE_NAME, vec![x, y]),
            PostgresType::Real => (REAL_TYPE_NAME, vec![]),
            PostgresType::DoublePrecision => (DOUBLE_PRECISION_TYPE_NAME, vec![]),
            PostgresType::SmallSerial => (SMALL_SERIAL_TYPE_NAME, vec![]),
            PostgresType::Serial => (SMALL_SERIAL_TYPE_NAME, vec![]),
            PostgresType::BigSerial => (BIG_SERIAL_TYPE_NAME, vec![]),
            PostgresType::VarChar(x) => (VARCHAR_TYPE_NAME, vec![x]),
            PostgresType::Char(x) => (CHAR_TYPE_NAME, vec![x]),
            PostgresType::Text => (TEXT_TYPE_NAME, vec![]),
            PostgresType::ByteA => (BYTE_A_TYPE_NAME, vec![]),
            PostgresType::Timestamp(x) => (TIMESTAMP_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::TimestampWithTimeZone(x) => (TIMESTAMP_WITH_TIMEZONE_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::Date => (DATE_TYPE_NAME, vec![]),
            PostgresType::Time(x) => (TIME_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::TimeWithTimeZone(x) => (TIME_WITH_TIMEZONE_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::Interval(x) => (INTERVAL_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::Boolean => (BOOLEAN_TYPE_NAME, vec![]),
            PostgresType::Bit(x) => (BIT_TYPE_NAME, vec![x]),
            PostgresType::VarBit(x) => (VAR_BIT_TYPE_NAME, vec![x]),
            PostgresType::UUID => (UUID_TYPE_NAME, vec![]),
            PostgresType::Xml => (XML_TYPE_NAME, vec![]),
            PostgresType::JSON => (JSON_TYPE_NAME, vec![]),
            PostgresType::JSONB => (JSON_B_TYPE_NAME, vec![]),
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
                connector_name: "Postgres".parse().unwrap(),
            }))
        }
    }
}
