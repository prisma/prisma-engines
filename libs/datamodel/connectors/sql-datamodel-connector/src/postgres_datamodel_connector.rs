use datamodel_connector::connector_error::{ConnectorError, ErrorKind};
use datamodel_connector::helper::{arg_vec_from_opt, args_vec_from_opt, parse_u32_arguments};
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
const DATE_TYPE_NAME: &str = "Date";
const TIME_TYPE_NAME: &str = "Time";
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
            ConnectorCapability::RelationFieldsInArbitraryOrder,
        ];

        let small_int = NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, vec![ScalarType::Int]);
        let integer = NativeTypeConstructor::without_args(INTEGER_TYPE_NAME, vec![ScalarType::Int]);
        let big_int = NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, vec![ScalarType::BigInt]);
        let decimal = NativeTypeConstructor::with_optional_args(DECIMAL_TYPE_NAME, 2, vec![ScalarType::Decimal]);
        let numeric = NativeTypeConstructor::with_optional_args(NUMERIC_TYPE_NAME, 2, vec![ScalarType::Decimal]);
        let real = NativeTypeConstructor::without_args(REAL_TYPE_NAME, vec![ScalarType::Float]);
        let double_precision = NativeTypeConstructor::without_args(DOUBLE_PRECISION_TYPE_NAME, vec![ScalarType::Float]);
        let small_serial = NativeTypeConstructor::without_args(SMALL_SERIAL_TYPE_NAME, vec![ScalarType::Int]);
        let serial = NativeTypeConstructor::without_args(SERIAL_TYPE_NAME, vec![ScalarType::Int]);
        let big_serial = NativeTypeConstructor::without_args(BIG_SERIAL_TYPE_NAME, vec![ScalarType::Int]);
        let varchar = NativeTypeConstructor::with_optional_args(VARCHAR_TYPE_NAME, 1, vec![ScalarType::String]);
        let char = NativeTypeConstructor::with_optional_args(CHAR_TYPE_NAME, 1, vec![ScalarType::String]);
        let text = NativeTypeConstructor::without_args(TEXT_TYPE_NAME, vec![ScalarType::String]);
        let byte_a = NativeTypeConstructor::without_args(BYTE_A_TYPE_NAME, vec![ScalarType::Bytes]);
        let timestamp = NativeTypeConstructor::with_optional_args(TIMESTAMP_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let date = NativeTypeConstructor::without_args(DATE_TYPE_NAME, vec![ScalarType::DateTime]);
        let time = NativeTypeConstructor::with_optional_args(TIME_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let boolean = NativeTypeConstructor::without_args(BOOLEAN_TYPE_NAME, vec![ScalarType::Boolean]);
        let bit = NativeTypeConstructor::with_optional_args(BIT_TYPE_NAME, 1, vec![ScalarType::String]);
        let varbit = NativeTypeConstructor::with_optional_args(VAR_BIT_TYPE_NAME, 1, vec![ScalarType::String]);
        let uuid = NativeTypeConstructor::without_args(UUID_TYPE_NAME, vec![ScalarType::String]);
        let xml = NativeTypeConstructor::without_args(XML_TYPE_NAME, vec![ScalarType::String]);
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
            date,
            time,
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
        if let FieldType::NativeType(_scalar_type, native_type_instance) = field.field_type() {
            let native_type_name = native_type_instance.name.as_str();
            let native_type: PostgresType = native_type_instance.deserialize_native_type();

            let precision_and_scale = match native_type {
                PostgresType::Decimal(x) => x,
                PostgresType::Numeric(x) => x,
                _ => None,
            };
            match precision_and_scale {
                Some((precision, scale)) if scale > precision => {
                    return Err(ConnectorError::new_scale_larger_than_precision_error(
                        native_type_name,
                        "Postgres",
                    ));
                }
                Some((precision, _)) if precision > 1000 || precision <= 0 => {
                    return Err(ConnectorError::new_argument_m_out_of_range_error(
                        "Precision must be positive with a maximum value of 1000.",
                        native_type_name,
                        "Postgres",
                    ));
                }
                _ => {}
            }

            let length = match native_type {
                PostgresType::Bit(l) => l,
                PostgresType::VarBit(l) => l,
                _ => None,
            };
            if length == Some(0) {
                return Err(ConnectorError::new_argument_m_out_of_range_error(
                    "M must be a positive integer.",
                    native_type_name,
                    "Postgres",
                ));
            }

            if matches!(
                native_type,
                PostgresType::SmallSerial | PostgresType::Serial | PostgresType::BigSerial
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

            let time_precision = match native_type {
                PostgresType::Timestamp(p) => p,
                PostgresType::Time(p) => p,
                _ => None,
            };

            if let Some(precision) = time_precision {
                if precision > 6 {
                    return Err(ConnectorError::new_argument_m_out_of_range_error(
                        "M can range from 0 to 6.",
                        native_type_name,
                        "Postgres",
                    ));
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
                [precision, scale] => PostgresType::Decimal(Some((*precision, *scale))),
                [] => PostgresType::Decimal(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(DECIMAL_TYPE_NAME, 2, parsed_args.len())),
            },
            NUMERIC_TYPE_NAME => match parsed_args.as_slice() {
                [scale, precision] => PostgresType::Numeric(Some((*scale, *precision))),
                [] => PostgresType::Numeric(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(NUMERIC_TYPE_NAME, 2, parsed_args.len())),
            },
            REAL_TYPE_NAME => PostgresType::Real,
            DOUBLE_PRECISION_TYPE_NAME => PostgresType::DoublePrecision,
            SMALL_SERIAL_TYPE_NAME => PostgresType::SmallSerial,
            SERIAL_TYPE_NAME => PostgresType::Serial,
            BIG_SERIAL_TYPE_NAME => PostgresType::BigSerial,
            VARCHAR_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::VarChar(Some(*arg)),
                [] => PostgresType::VarChar(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(VARCHAR_TYPE_NAME, 1, parsed_args.len())),
            },
            CHAR_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::Char(Some(*arg)),
                [] => PostgresType::Char(None),
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
            BOOLEAN_TYPE_NAME => PostgresType::Boolean,
            BIT_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::Bit(Some(*arg)),
                [] => PostgresType::Bit(None),
                _ => return Err(self.wrap_in_argument_count_mismatch_error(BIT_TYPE_NAME, 1, parsed_args.len())),
            },
            VAR_BIT_TYPE_NAME => match parsed_args.as_slice() {
                [arg] => PostgresType::VarBit(Some(*arg)),
                [] => PostgresType::VarBit(None),
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
            PostgresType::Decimal(x) => (DECIMAL_TYPE_NAME, args_vec_from_opt(x)),
            PostgresType::Numeric(x) => (NUMERIC_TYPE_NAME, args_vec_from_opt(x)),
            PostgresType::Real => (REAL_TYPE_NAME, vec![]),
            PostgresType::DoublePrecision => (DOUBLE_PRECISION_TYPE_NAME, vec![]),
            PostgresType::SmallSerial => (SMALL_SERIAL_TYPE_NAME, vec![]),
            PostgresType::Serial => (SMALL_SERIAL_TYPE_NAME, vec![]),
            PostgresType::BigSerial => (BIG_SERIAL_TYPE_NAME, vec![]),
            PostgresType::VarChar(x) => (VARCHAR_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::Char(x) => (CHAR_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::Text => (TEXT_TYPE_NAME, vec![]),
            PostgresType::ByteA => (BYTE_A_TYPE_NAME, vec![]),
            PostgresType::Timestamp(x) => (TIMESTAMP_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::Date => (DATE_TYPE_NAME, vec![]),
            PostgresType::Time(x) => (TIME_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::Boolean => (BOOLEAN_TYPE_NAME, vec![]),
            PostgresType::Bit(x) => (BIT_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::VarBit(x) => (VAR_BIT_TYPE_NAME, arg_vec_from_opt(x)),
            PostgresType::UUID => (UUID_TYPE_NAME, vec![]),
            PostgresType::Xml => (XML_TYPE_NAME, vec![]),
            PostgresType::JSON => (JSON_TYPE_NAME, vec![]),
            PostgresType::JSONB => (JSON_B_TYPE_NAME, vec![]),
        };

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
