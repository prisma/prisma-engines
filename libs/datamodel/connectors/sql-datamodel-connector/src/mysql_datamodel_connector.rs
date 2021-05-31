use datamodel_connector::connector_error::ConnectorError;
use datamodel_connector::helper::{args_vec_from_opt, parse_one_opt_u32, parse_one_u32, parse_two_opt_u32};
use datamodel_connector::{Connector, ConnectorCapability};
use dml::field::{Field, FieldType};
use dml::model::{IndexType, Model};
use dml::native_type_constructor::NativeTypeConstructor;
use dml::native_type_instance::NativeTypeInstance;
use dml::scalars::ScalarType;
use native_types::MySqlType;
use native_types::MySqlType::*;

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
const DATETIME_TYPE_NAME: &str = "DateTime";
const TIMESTAMP_TYPE_NAME: &str = "Timestamp";
const YEAR_TYPE_NAME: &str = "Year";
const JSON_TYPE_NAME: &str = "Json";

const NATIVE_TYPES_THAT_CAN_NOT_BE_USED_IN_KEY_SPECIFICATION: &[&str] = &[
    TEXT_TYPE_NAME,
    LONG_TEXT_TYPE_NAME,
    MEDIUM_TEXT_TYPE_NAME,
    TINY_TEXT_TYPE_NAME,
    BLOB_TYPE_NAME,
    TINY_BLOB_TYPE_NAME,
    MEDIUM_BLOB_TYPE_NAME,
    LONG_BLOB_TYPE_NAME,
];

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
            ConnectorCapability::RelationFieldsInArbitraryOrder,
            ConnectorCapability::CreateMany,
            ConnectorCapability::WritableAutoincField,
            ConnectorCapability::CreateSkipDuplicates,
            ConnectorCapability::UpdateableId,
            ConnectorCapability::JsonFilteringJsonPath,
            ConnectorCapability::CreateManyWriteableAutoIncId,
            ConnectorCapability::AutoIncrement,
            ConnectorCapability::CompoundIds,
        ];

        let int = NativeTypeConstructor::without_args(INT_TYPE_NAME, vec![ScalarType::Int]);
        let unsigned_int = NativeTypeConstructor::without_args(UNSIGNED_INT_TYPE_NAME, vec![ScalarType::Int]);
        let small_int = NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, vec![ScalarType::Int]);
        let unsigned_small_int =
            NativeTypeConstructor::without_args(UNSIGNED_SMALL_INT_TYPE_NAME, vec![ScalarType::Int]);
        let tiny_int =
            NativeTypeConstructor::without_args(TINY_INT_TYPE_NAME, vec![ScalarType::Boolean, ScalarType::Int]);
        let unsigned_tiny_int = NativeTypeConstructor::without_args(UNSIGNED_TINY_INT_TYPE_NAME, vec![ScalarType::Int]);
        let medium_int = NativeTypeConstructor::without_args(MEDIUM_INT_TYPE_NAME, vec![ScalarType::Int]);
        let unsigned_medium_int =
            NativeTypeConstructor::without_args(UNSIGNED_MEDIUM_INT_TYPE_NAME, vec![ScalarType::Int]);
        let big_int = NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, vec![ScalarType::BigInt]);
        let unsigned_big_int =
            NativeTypeConstructor::without_args(UNSIGNED_BIG_INT_TYPE_NAME, vec![ScalarType::BigInt]);
        let decimal = NativeTypeConstructor::with_optional_args(DECIMAL_TYPE_NAME, 2, vec![ScalarType::Decimal]);
        let float = NativeTypeConstructor::without_args(FLOAT_TYPE_NAME, vec![ScalarType::Float]);
        let double = NativeTypeConstructor::without_args(DOUBLE_TYPE_NAME, vec![ScalarType::Float]);
        let bit = NativeTypeConstructor::with_args(BIT_TYPE_NAME, 1, vec![ScalarType::Boolean, ScalarType::Bytes]);
        let char = NativeTypeConstructor::with_args(CHAR_TYPE_NAME, 1, vec![ScalarType::String]);
        let var_char = NativeTypeConstructor::with_args(VAR_CHAR_TYPE_NAME, 1, vec![ScalarType::String]);
        let binary = NativeTypeConstructor::with_args(BINARY_TYPE_NAME, 1, vec![ScalarType::Bytes]);
        let var_binary = NativeTypeConstructor::with_args(VAR_BINARY_TYPE_NAME, 1, vec![ScalarType::Bytes]);
        let tiny_blob = NativeTypeConstructor::without_args(TINY_BLOB_TYPE_NAME, vec![ScalarType::Bytes]);
        let blob = NativeTypeConstructor::without_args(BLOB_TYPE_NAME, vec![ScalarType::Bytes]);
        let medium_blob = NativeTypeConstructor::without_args(MEDIUM_BLOB_TYPE_NAME, vec![ScalarType::Bytes]);
        let long_blob = NativeTypeConstructor::without_args(LONG_BLOB_TYPE_NAME, vec![ScalarType::Bytes]);
        let tiny_text = NativeTypeConstructor::without_args(TINY_TEXT_TYPE_NAME, vec![ScalarType::String]);
        let text = NativeTypeConstructor::without_args(TEXT_TYPE_NAME, vec![ScalarType::String]);
        let medium_text = NativeTypeConstructor::without_args(MEDIUM_TEXT_TYPE_NAME, vec![ScalarType::String]);
        let long_text = NativeTypeConstructor::without_args(LONG_TEXT_TYPE_NAME, vec![ScalarType::String]);
        let date = NativeTypeConstructor::without_args(DATE_TYPE_NAME, vec![ScalarType::DateTime]);
        let time = NativeTypeConstructor::with_optional_args(TIME_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let datetime = NativeTypeConstructor::with_optional_args(DATETIME_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let timestamp = NativeTypeConstructor::with_optional_args(TIMESTAMP_TYPE_NAME, 1, vec![ScalarType::DateTime]);
        let year = NativeTypeConstructor::without_args(YEAR_TYPE_NAME, vec![ScalarType::Int]);
        let json = NativeTypeConstructor::without_args(JSON_TYPE_NAME, vec![ScalarType::Json]);

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

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, MySqlType)] = &[
    (ScalarType::Int, MySqlType::Int),
    (ScalarType::BigInt, MySqlType::BigInt),
    (ScalarType::Float, MySqlType::Double),
    (ScalarType::Decimal, MySqlType::Decimal(Some((65, 30)))),
    (ScalarType::Boolean, MySqlType::TinyInt),
    (ScalarType::String, MySqlType::VarChar(191)),
    (ScalarType::DateTime, MySqlType::DateTime(Some(3))),
    (ScalarType::Bytes, MySqlType::LongBlob),
    (ScalarType::Json, MySqlType::Json),
];

impl Connector for MySqlDatamodelConnector {
    fn name(&self) -> String {
        "MySQL".to_string()
    }

    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn scalar_type_for_native_type(&self, native_type: serde_json::Value) -> ScalarType {
        let native_type: MySqlType = serde_json::from_value(native_type).unwrap();

        match native_type {
            //String
            VarChar(_) => ScalarType::String,
            Text => ScalarType::String,
            Char(_) => ScalarType::String,
            TinyText => ScalarType::String,
            MediumText => ScalarType::String,
            LongText => ScalarType::String,
            //Boolean
            Bit(1) => ScalarType::Bytes,
            //Int
            Int => ScalarType::Int,
            SmallInt => ScalarType::Int,
            MediumInt => ScalarType::Int,
            Year => ScalarType::Int,
            TinyInt => ScalarType::Int,
            //BigInt
            BigInt => ScalarType::BigInt,
            //Float
            Float => ScalarType::Float,
            Double => ScalarType::Float,
            //Decimal
            Decimal(_) => ScalarType::Decimal,
            //DateTime
            DateTime(_) => ScalarType::DateTime,
            Date => ScalarType::DateTime,
            Time(_) => ScalarType::DateTime,
            Timestamp(_) => ScalarType::DateTime,
            //Json
            Json => ScalarType::Json,
            //Bytes
            LongBlob => ScalarType::Bytes,
            Binary(_) => ScalarType::Bytes,
            VarBinary(_) => ScalarType::Bytes,
            TinyBlob => ScalarType::Bytes,
            Blob => ScalarType::Bytes,
            MediumBlob => ScalarType::Bytes,
            Bit(_) => ScalarType::Bytes,
            //Missing from docs
            UnsignedInt => ScalarType::Int,
            UnsignedSmallInt => ScalarType::Int,
            UnsignedTinyInt => ScalarType::Int,
            UnsignedMediumInt => ScalarType::Int,
            UnsignedBigInt => ScalarType::BigInt,
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let native_type = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .ok_or_else(|| format!("Could not find scalar type {:?} in SCALAR_TYPE_DEFAULTS", scalar_type))
            .unwrap();

        serde_json::to_value(native_type).expect("MySqlType to JSON failed")
    }

    fn native_type_is_default_for_scalar_type(&self, native_type: serde_json::Value, scalar_type: &ScalarType) -> bool {
        let native_type: MySqlType = serde_json::from_value(native_type).expect("MySqlType from JSON failed");

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && &native_type == nt)
    }

    fn validate_field(&self, field: &Field) -> Result<(), ConnectorError> {
        match field.field_type() {
            FieldType::NativeType(scalar_type, native_type_instance) => {
                let native_type_name = native_type_instance.name.as_str();
                let native_type: MySqlType = native_type_instance.deserialize_native_type();
                let error = self.native_instance_error(native_type_instance.clone());
                let incompatible_with_key =
                    NATIVE_TYPES_THAT_CAN_NOT_BE_USED_IN_KEY_SPECIFICATION.contains(&native_type_name);

                match native_type {
                    Decimal(Some((precision, scale))) if scale > precision => {
                        error.new_scale_larger_than_precision_error()
                    }
                    Decimal(Some((precision, _))) if precision > 65 => {
                        error.new_argument_m_out_of_range_error("Precision can range from 1 to 65.")
                    }
                    Decimal(Some((_, scale))) if scale > 30 => {
                        error.new_argument_m_out_of_range_error("Scale can range from 0 to 30.")
                    }
                    Bit(length) if length == 0 || length > 64 => {
                        error.new_argument_m_out_of_range_error("M can range from 1 to 64.")
                    }
                    Char(length) if length > 255 => {
                        error.new_argument_m_out_of_range_error("M can range from 0 to 255.")
                    }
                    VarChar(length) if length > 65535 => {
                        error.new_argument_m_out_of_range_error("M can range from 0 to 65,535.")
                    }
                    Bit(n) if n > 1 && scalar_type.is_boolean() => {
                        error.new_argument_m_out_of_range_error("only Bit(1) can be used as Boolean.")
                    }
                    _ if field.is_unique() && incompatible_with_key => error.new_incompatible_native_type_with_unique(),
                    _ if field.is_id() && incompatible_with_key => error.new_incompatible_native_type_with_id(),
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    fn validate_model(&self, model: &Model) -> Result<(), ConnectorError> {
        for index_definition in model.indices.iter() {
            let fields = index_definition.fields.iter().map(|f| model.find_field(f).unwrap());
            for f in fields {
                if let FieldType::NativeType(_, native_type) = f.field_type() {
                    let native_type_name = native_type.name.as_str();

                    if NATIVE_TYPES_THAT_CAN_NOT_BE_USED_IN_KEY_SPECIFICATION.contains(&native_type_name) {
                        return if index_definition.tpe == IndexType::Unique {
                            self.native_instance_error(native_type.clone())
                                .new_incompatible_native_type_with_unique()
                        } else {
                            self.native_instance_error(native_type.clone())
                                .new_incompatible_native_type_with_index()
                        };
                    }
                }
            }
        }
        for id_field in model.id_fields.iter() {
            let field = model.find_field(id_field).unwrap();
            if let FieldType::NativeType(_, native_type) = field.field_type() {
                let native_type_name = native_type.name.as_str();
                if NATIVE_TYPES_THAT_CAN_NOT_BE_USED_IN_KEY_SPECIFICATION.contains(&native_type_name) {
                    return self
                        .native_instance_error(native_type.clone())
                        .new_incompatible_native_type_with_id();
                }
            }
        }
        Ok(())
    }

    fn available_native_type_constructors(&self) -> &[NativeTypeConstructor] {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError> {
        let cloned_args = args.clone();

        let native_type = match name {
            INT_TYPE_NAME => Int,
            UNSIGNED_INT_TYPE_NAME => UnsignedInt,
            SMALL_INT_TYPE_NAME => SmallInt,
            UNSIGNED_SMALL_INT_TYPE_NAME => UnsignedSmallInt,
            TINY_INT_TYPE_NAME => TinyInt,
            UNSIGNED_TINY_INT_TYPE_NAME => UnsignedTinyInt,
            MEDIUM_INT_TYPE_NAME => MediumInt,
            UNSIGNED_MEDIUM_INT_TYPE_NAME => UnsignedMediumInt,
            BIG_INT_TYPE_NAME => BigInt,
            UNSIGNED_BIG_INT_TYPE_NAME => UnsignedBigInt,
            DECIMAL_TYPE_NAME => Decimal(parse_two_opt_u32(args, DECIMAL_TYPE_NAME)?),
            FLOAT_TYPE_NAME => Float,
            DOUBLE_TYPE_NAME => Double,
            BIT_TYPE_NAME => Bit(parse_one_u32(args, BIT_TYPE_NAME)?),
            CHAR_TYPE_NAME => Char(parse_one_u32(args, CHAR_TYPE_NAME)?),
            VAR_CHAR_TYPE_NAME => VarChar(parse_one_u32(args, VAR_CHAR_TYPE_NAME)?),
            BINARY_TYPE_NAME => Binary(parse_one_u32(args, BINARY_TYPE_NAME)?),
            VAR_BINARY_TYPE_NAME => VarBinary(parse_one_u32(args, VAR_BINARY_TYPE_NAME)?),
            TINY_BLOB_TYPE_NAME => TinyBlob,
            BLOB_TYPE_NAME => Blob,
            MEDIUM_BLOB_TYPE_NAME => MediumBlob,
            LONG_BLOB_TYPE_NAME => LongBlob,
            TINY_TEXT_TYPE_NAME => TinyText,
            TEXT_TYPE_NAME => Text,
            MEDIUM_TEXT_TYPE_NAME => MediumText,
            LONG_TEXT_TYPE_NAME => LongText,
            DATE_TYPE_NAME => Date,
            TIME_TYPE_NAME => Time(parse_one_opt_u32(args, TIME_TYPE_NAME)?),
            DATETIME_TYPE_NAME => DateTime(parse_one_opt_u32(args, DATETIME_TYPE_NAME)?),
            TIMESTAMP_TYPE_NAME => Timestamp(parse_one_opt_u32(args, TIMESTAMP_TYPE_NAME)?),
            YEAR_TYPE_NAME => Year,
            JSON_TYPE_NAME => Json,
            _ => return Err(ConnectorError::new_native_type_parser_error(name)),
        };

        Ok(NativeTypeInstance::new(name, cloned_args, &native_type))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        let native_type: MySqlType = serde_json::from_value(native_type).unwrap();
        let (constructor_name, args) = match native_type {
            Int => (INT_TYPE_NAME, vec![]),
            UnsignedInt => (UNSIGNED_INT_TYPE_NAME, vec![]),
            SmallInt => (SMALL_INT_TYPE_NAME, vec![]),
            UnsignedSmallInt => (UNSIGNED_SMALL_INT_TYPE_NAME, vec![]),
            TinyInt => (TINY_INT_TYPE_NAME, vec![]),
            UnsignedTinyInt => (UNSIGNED_TINY_INT_TYPE_NAME, vec![]),
            MediumInt => (MEDIUM_INT_TYPE_NAME, vec![]),
            UnsignedMediumInt => (UNSIGNED_MEDIUM_INT_TYPE_NAME, vec![]),
            BigInt => (BIG_INT_TYPE_NAME, vec![]),
            UnsignedBigInt => (UNSIGNED_BIG_INT_TYPE_NAME, vec![]),
            Decimal(x) => (DECIMAL_TYPE_NAME, args_vec_from_opt(x)),
            Float => (FLOAT_TYPE_NAME, vec![]),
            Double => (DOUBLE_TYPE_NAME, vec![]),
            Bit(x) => (BIT_TYPE_NAME, vec![x.to_string()]),
            Char(x) => (CHAR_TYPE_NAME, vec![x.to_string()]),
            VarChar(x) => (VAR_CHAR_TYPE_NAME, vec![x.to_string()]),
            Binary(x) => (BINARY_TYPE_NAME, vec![x.to_string()]),
            VarBinary(x) => (VAR_BINARY_TYPE_NAME, vec![x.to_string()]),
            TinyBlob => (TINY_BLOB_TYPE_NAME, vec![]),
            Blob => (BLOB_TYPE_NAME, vec![]),
            MediumBlob => (MEDIUM_BLOB_TYPE_NAME, vec![]),
            LongBlob => (LONG_BLOB_TYPE_NAME, vec![]),
            TinyText => (TINY_TEXT_TYPE_NAME, vec![]),
            Text => (TEXT_TYPE_NAME, vec![]),
            MediumText => (MEDIUM_TEXT_TYPE_NAME, vec![]),
            LongText => (LONG_TEXT_TYPE_NAME, vec![]),
            Date => (DATE_TYPE_NAME, vec![]),
            Time(x) => (TIME_TYPE_NAME, arg_vec_from_opt(x)),
            DateTime(x) => (DATETIME_TYPE_NAME, arg_vec_from_opt(x)),
            Timestamp(x) => (TIMESTAMP_TYPE_NAME, arg_vec_from_opt(x)),
            Year => (YEAR_TYPE_NAME, vec![]),
            Json => (JSON_TYPE_NAME, vec![]),
        };

        fn arg_vec_from_opt(input: Option<u32>) -> Vec<String> {
            match input {
                Some(arg) => vec![arg.to_string()],
                None => vec![],
            }
        }

        if let Some(constructor) = self.find_native_type_constructor(constructor_name) {
            Ok(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type))
        } else {
            self.native_str_error(constructor_name).native_type_name_unknown()
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("mysql") {
            return Err("must start with the protocol `mysql://`.".to_owned());
        }

        Ok(())
    }
}

impl Default for MySqlDatamodelConnector {
    fn default() -> Self {
        Self::new()
    }
}
