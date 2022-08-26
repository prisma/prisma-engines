mod validations;

use datamodel_connector::{
    helper::{args_vec_from_opt, parse_one_opt_u32, parse_one_u32, parse_two_opt_u32},
    parser_database::walkers,
    Connector, ConnectorCapability, ConstraintScope, DatamodelError, Diagnostics, NativeTypeConstructor,
    NativeTypeInstance, ReferentialAction, ReferentialIntegrity, ScalarType, Span,
};
use enumflags2::BitFlags;
use native_types::{
    MySqlType::{self, *},
    NativeType,
};

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

const NATIVE_TYPE_CONSTRUCTORS: &[NativeTypeConstructor] = &[
    NativeTypeConstructor::without_args(INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(UNSIGNED_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(SMALL_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(UNSIGNED_SMALL_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(TINY_INT_TYPE_NAME, &[ScalarType::Boolean, ScalarType::Int]),
    NativeTypeConstructor::without_args(UNSIGNED_TINY_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(MEDIUM_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(UNSIGNED_MEDIUM_INT_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, &[ScalarType::BigInt]),
    NativeTypeConstructor::without_args(UNSIGNED_BIG_INT_TYPE_NAME, &[ScalarType::BigInt]),
    NativeTypeConstructor::with_optional_args(DECIMAL_TYPE_NAME, 2, &[ScalarType::Decimal]),
    NativeTypeConstructor::without_args(FLOAT_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(DOUBLE_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::with_args(BIT_TYPE_NAME, 1, &[ScalarType::Boolean, ScalarType::Bytes]),
    NativeTypeConstructor::with_args(CHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_args(VAR_CHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_args(BINARY_TYPE_NAME, 1, &[ScalarType::Bytes]),
    NativeTypeConstructor::with_args(VAR_BINARY_TYPE_NAME, 1, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(TINY_BLOB_TYPE_NAME, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(BLOB_TYPE_NAME, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(MEDIUM_BLOB_TYPE_NAME, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(LONG_BLOB_TYPE_NAME, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(TINY_TEXT_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(TEXT_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(MEDIUM_TEXT_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(LONG_TEXT_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(DATE_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(TIME_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(DATETIME_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(TIMESTAMP_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(YEAR_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(JSON_TYPE_NAME, &[ScalarType::Json]),
];

const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::Enums,
    ConnectorCapability::EnumArrayPush,
    ConnectorCapability::Json,
    ConnectorCapability::AutoIncrementAllowedOnNonId,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::CreateMany,
    ConnectorCapability::WritableAutoincField,
    ConnectorCapability::CreateSkipDuplicates,
    ConnectorCapability::UpdateableId,
    ConnectorCapability::JsonFiltering,
    ConnectorCapability::JsonFilteringJsonPath,
    ConnectorCapability::JsonFilteringAlphanumeric,
    ConnectorCapability::CreateManyWriteableAutoIncId,
    ConnectorCapability::AutoIncrement,
    ConnectorCapability::CompoundIds,
    ConnectorCapability::AnyId,
    ConnectorCapability::SqlQueryRaw,
    ConnectorCapability::NamedForeignKeys,
    ConnectorCapability::AdvancedJsonNullability,
    ConnectorCapability::IndexColumnLengthPrefixing,
    ConnectorCapability::MultiSchema,
    ConnectorCapability::FullTextIndex,
    ConnectorCapability::FullTextSearchWithIndex,
    ConnectorCapability::MultipleFullTextAttributesPerModel,
    ConnectorCapability::ImplicitManyToManyRelation,
    ConnectorCapability::DecimalType,
    ConnectorCapability::OrderByNullsFirstLast,
    ConnectorCapability::SupportsTxIsolationReadUncommitted,
    ConnectorCapability::SupportsTxIsolationReadCommitted,
    ConnectorCapability::SupportsTxIsolationRepeatableRead,
    ConnectorCapability::SupportsTxIsolationSerializable,
];

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::GlobalForeignKey, ConstraintScope::ModelKeyIndex];

pub struct MySqlDatamodelConnector;

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
    fn provider_name(&self) -> &'static str {
        "mysql"
    }

    fn name(&self) -> &str {
        "MySQL"
    }

    fn capabilities(&self) -> &'static [ConnectorCapability] {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        64
    }

    fn referential_actions(&self, referential_integrity: &ReferentialIntegrity) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        referential_integrity.allowed_referential_actions(Restrict | Cascade | SetNull | NoAction | SetDefault)
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

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        scalar_type: &ScalarType,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: MySqlType =
            serde_json::from_value(native_type_instance.serialized_native_type.clone()).unwrap();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            Decimal(Some((precision, scale))) if scale > precision => {
                errors.push_error(error.new_scale_larger_than_precision_error(span))
            }
            Decimal(Some((precision, _))) if precision > 65 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision can range from 1 to 65.", span))
            }
            Decimal(Some((_, scale))) if scale > 30 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Scale can range from 0 to 30.", span))
            }
            Bit(length) if length == 0 || length > 64 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 1 to 64.", span))
            }
            Char(length) if length > 255 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 255.", span))
            }
            VarChar(length) if length > 65535 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 65,535.", span))
            }
            Bit(n) if n > 1 && matches!(scalar_type, ScalarType::Boolean) => {
                errors.push_error(error.new_argument_m_out_of_range_error("only Bit(1) can be used as Boolean.", span))
            }
            _ => (),
        }
    }

    fn validate_enum(&self, r#enum: walkers::EnumWalker<'_>, diagnostics: &mut Diagnostics) {
        if let Some((_, span)) = r#enum.schema() {
            diagnostics.push_error(DatamodelError::new_static(
                "MySQL enums do not belong to a schema.",
                span,
            ));
        }
    }

    fn validate_model(&self, model: walkers::ModelWalker<'_>, errors: &mut Diagnostics) {
        for index in model.indexes() {
            validations::field_types_can_be_used_in_an_index(self, index, errors);
        }

        if let Some(pk) = model.primary_key() {
            validations::field_types_can_be_used_in_a_primary_key(self, pk, errors);
        }
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        CONSTRAINT_SCOPES
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        NATIVE_TYPE_CONSTRUCTORS
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: Vec<String>,
        span: Span,
    ) -> Result<NativeTypeInstance, DatamodelError> {
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
            DECIMAL_TYPE_NAME => Decimal(parse_two_opt_u32(args, DECIMAL_TYPE_NAME, span)?),
            FLOAT_TYPE_NAME => Float,
            DOUBLE_TYPE_NAME => Double,
            BIT_TYPE_NAME => Bit(parse_one_u32(args, BIT_TYPE_NAME, span)?),
            CHAR_TYPE_NAME => Char(parse_one_u32(args, CHAR_TYPE_NAME, span)?),
            VAR_CHAR_TYPE_NAME => VarChar(parse_one_u32(args, VAR_CHAR_TYPE_NAME, span)?),
            BINARY_TYPE_NAME => Binary(parse_one_u32(args, BINARY_TYPE_NAME, span)?),
            VAR_BINARY_TYPE_NAME => VarBinary(parse_one_u32(args, VAR_BINARY_TYPE_NAME, span)?),
            TINY_BLOB_TYPE_NAME => TinyBlob,
            BLOB_TYPE_NAME => Blob,
            MEDIUM_BLOB_TYPE_NAME => MediumBlob,
            LONG_BLOB_TYPE_NAME => LongBlob,
            TINY_TEXT_TYPE_NAME => TinyText,
            TEXT_TYPE_NAME => Text,
            MEDIUM_TEXT_TYPE_NAME => MediumText,
            LONG_TEXT_TYPE_NAME => LongText,
            DATE_TYPE_NAME => Date,
            TIME_TYPE_NAME => Time(parse_one_opt_u32(args, TIME_TYPE_NAME, span)?),
            DATETIME_TYPE_NAME => DateTime(parse_one_opt_u32(args, DATETIME_TYPE_NAME, span)?),
            TIMESTAMP_TYPE_NAME => Timestamp(parse_one_opt_u32(args, TIMESTAMP_TYPE_NAME, span)?),
            YEAR_TYPE_NAME => Year,
            JSON_TYPE_NAME => Json,
            _ => return Err(DatamodelError::new_native_type_parser_error(name, span)),
        };

        Ok(NativeTypeInstance::new(name, cloned_args, native_type.to_json()))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> NativeTypeInstance {
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
            NativeTypeInstance::new(constructor.name, args, native_type.to_json())
        } else {
            unreachable!()
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("mysql") {
            return Err("must start with the protocol `mysql://`.".to_owned());
        }

        Ok(())
    }
}
