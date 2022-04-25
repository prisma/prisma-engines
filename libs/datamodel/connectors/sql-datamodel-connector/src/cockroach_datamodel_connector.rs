use datamodel_connector::{
    helper::{arg_vec_from_opt, args_vec_from_opt, parse_one_opt_u32, parse_two_opt_u32},
    parser_database, Connector, ConnectorCapability, ConstraintScope, DatamodelError, Diagnostics,
    NativeTypeConstructor, NativeTypeInstance, ReferentialAction, ReferentialIntegrity, ScalarType, Span,
};
use enumflags2::BitFlags;
use native_types::{CockroachType, NativeType};

const BIT_TYPE_NAME: &str = "Bit";
const BOOL_TYPE_NAME: &str = "Bool";
const BYTES_TYPE_NAME: &str = "Bytes";
const CHAR_TYPE_NAME: &str = "Char";
const DATE_TYPE_NAME: &str = "Date";
const DECIMAL_TYPE_NAME: &str = "Decimal";
const FLOAT4_TYPE_NAME: &str = "Float4";
const FLOAT8_TYPE_NAME: &str = "Float8";
const INET_TYPE_NAME: &str = "Inet";
const INT2_TYPE_NAME: &str = "Int2";
const INT4_TYPE_NAME: &str = "Int4";
const INT8_TYPE_NAME: &str = "Int8";
const JSON_B_TYPE_NAME: &str = "JsonB";
const SINGLE_CHAR_TYPE_NAME: &str = "SingleChar";
const STRING_TYPE_NAME: &str = "String";
const TIMESTAMP_TYPE_NAME: &str = "Timestamp";
const TIMESTAMP_TZ_TYPE_NAME: &str = "Timestamptz";
const TIME_TYPE_NAME: &str = "Time";
const TIME_TZ_TYPE_NAME: &str = "Timetz";
const UUID_TYPE_NAME: &str = "Uuid";
const VAR_BIT_TYPE_NAME: &str = "VarBit";
const VARCHAR_TYPE_NAME: &str = "VarChar";

const NATIVE_TYPE_CONSTRUCTORS: &[NativeTypeConstructor] = &[
    NativeTypeConstructor::with_optional_args(BIT_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(CHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(DECIMAL_TYPE_NAME, 2, &[ScalarType::Decimal]),
    NativeTypeConstructor::with_optional_args(STRING_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(TIMESTAMP_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(TIMESTAMP_TZ_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(TIME_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(TIME_TZ_TYPE_NAME, 1, &[ScalarType::DateTime]),
    NativeTypeConstructor::with_optional_args(VAR_BIT_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::with_optional_args(VARCHAR_TYPE_NAME, 1, &[ScalarType::String]),
    NativeTypeConstructor::without_args(BOOL_TYPE_NAME, &[ScalarType::Boolean]),
    NativeTypeConstructor::without_args(BYTES_TYPE_NAME, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(DATE_TYPE_NAME, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(FLOAT4_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(FLOAT8_TYPE_NAME, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(INET_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(INT2_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(INT4_TYPE_NAME, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(INT8_TYPE_NAME, &[ScalarType::BigInt]),
    NativeTypeConstructor::without_args(JSON_B_TYPE_NAME, &[ScalarType::Json]),
    NativeTypeConstructor::without_args(SINGLE_CHAR_TYPE_NAME, &[ScalarType::String]),
    NativeTypeConstructor::without_args(UUID_TYPE_NAME, &[ScalarType::String]),
];

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey];

const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::AdvancedJsonNullability,
    ConnectorCapability::AnyId,
    ConnectorCapability::AutoIncrement,
    ConnectorCapability::AutoIncrementAllowedOnNonId,
    ConnectorCapability::AutoIncrementMultipleAllowed,
    ConnectorCapability::AutoIncrementNonIndexedAllowed,
    ConnectorCapability::CompoundIds,
    ConnectorCapability::CreateMany,
    ConnectorCapability::CreateManyWriteableAutoIncId,
    ConnectorCapability::CreateSkipDuplicates,
    ConnectorCapability::Enums,
    ConnectorCapability::InsensitiveFilters,
    ConnectorCapability::Json,
    ConnectorCapability::JsonFilteringArrayPath,
    ConnectorCapability::NamedPrimaryKeys,
    ConnectorCapability::NamedForeignKeys,
    ConnectorCapability::SqlQueryRaw,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::ScalarLists,
    ConnectorCapability::UpdateableId,
    ConnectorCapability::WritableAutoincField,
    ConnectorCapability::ImplicitManyToManyRelation,
    ConnectorCapability::DecimalType,
];

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, CockroachType)] = &[
    (ScalarType::Int, CockroachType::Int4),
    (ScalarType::BigInt, CockroachType::Int8),
    (ScalarType::Float, CockroachType::Float8),
    (ScalarType::Decimal, CockroachType::Decimal(Some((65, 30)))),
    (ScalarType::Boolean, CockroachType::Bool),
    (ScalarType::String, CockroachType::String(None)),
    (ScalarType::DateTime, CockroachType::Timestamp(Some(3))),
    (ScalarType::Bytes, CockroachType::Bytes),
    (ScalarType::Json, CockroachType::JsonB),
];

pub struct CockroachDatamodelConnector;

impl Connector for CockroachDatamodelConnector {
    fn name(&self) -> &str {
        "CockroachDB"
    }

    fn capabilities(&self) -> &'static [ConnectorCapability] {
        CAPABILITIES
    }

    /// The maximum length of postgres identifiers, in bytes.
    ///
    /// Reference: <https://www.postgresql.org/docs/12/limits.html>
    fn max_identifier_length(&self) -> usize {
        63
    }

    fn referential_actions(&self, referential_integrity: &ReferentialIntegrity) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        referential_integrity.allowed_referential_actions(NoAction | Restrict | Cascade | SetNull | SetDefault)
    }

    fn scalar_type_for_native_type(&self, native_type: serde_json::Value) -> ScalarType {
        let native_type: CockroachType = serde_json::from_value(native_type).unwrap();

        match native_type {
            // String
            CockroachType::Char(_) => ScalarType::String,
            CockroachType::SingleChar => ScalarType::String,
            CockroachType::String(_) => ScalarType::String,
            CockroachType::Bit(_) => ScalarType::String,
            CockroachType::VarBit(_) => ScalarType::String,
            CockroachType::Uuid => ScalarType::String,
            CockroachType::Inet => ScalarType::String,
            // Boolean
            CockroachType::Bool => ScalarType::Boolean,
            // Int
            CockroachType::Int2 => ScalarType::Int,
            CockroachType::Int4 => ScalarType::Int,
            // BigInt
            CockroachType::Int8 => ScalarType::BigInt,
            // Float
            CockroachType::Float4 => ScalarType::Float,
            CockroachType::Float8 => ScalarType::Float,
            // Decimal
            CockroachType::Decimal(_) => ScalarType::Decimal,
            // DateTime
            CockroachType::Timestamp(_) => ScalarType::DateTime,
            CockroachType::Timestamptz(_) => ScalarType::DateTime,
            CockroachType::Date => ScalarType::DateTime,
            CockroachType::Time(_) => ScalarType::DateTime,
            CockroachType::Timetz(_) => ScalarType::DateTime,
            // Json
            CockroachType::JsonB => ScalarType::Json,
            // Bytes
            CockroachType::Bytes => ScalarType::Bytes,
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let native_type = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .ok_or_else(|| format!("Could not find scalar type {:?} in SCALAR_TYPE_DEFAULTS", scalar_type))
            .unwrap();

        serde_json::to_value(native_type).expect("CockroachType to JSON failed")
    }

    fn native_type_is_default_for_scalar_type(&self, native_type: serde_json::Value, scalar_type: &ScalarType) -> bool {
        let native_type: CockroachType = serde_json::from_value(native_type).expect("CockroachType from JSON failed");

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && &native_type == nt)
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: CockroachType =
            serde_json::from_value(native_type_instance.serialized_native_type.clone()).unwrap();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            CockroachType::Decimal(Some((precision, scale))) if scale > precision => {
                errors.push_error(error.new_scale_larger_than_precision_error(span))
            }
            CockroachType::Decimal(Some((prec, _))) if prec > 1000 || prec == 0 => {
                errors.push_error(error.new_argument_m_out_of_range_error(
                    "Precision must be positive with a maximum value of 1000.",
                    span,
                ))
            }
            CockroachType::Bit(Some(0)) | CockroachType::VarBit(Some(0)) => {
                errors.push_error(error.new_argument_m_out_of_range_error("M must be a positive integer.", span))
            }
            CockroachType::Timestamp(Some(p))
            | CockroachType::Timestamptz(Some(p))
            | CockroachType::Time(Some(p))
            | CockroachType::Timetz(Some(p))
                if p > 6 =>
            {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 6.", span))
            }
            _ => (),
        }
    }

    fn validate_model(&self, _model: parser_database::walkers::ModelWalker<'_>, _diagnostics: &mut Diagnostics) {}

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
            BYTES_TYPE_NAME => CockroachType::Bytes,
            CHAR_TYPE_NAME => CockroachType::Char(parse_one_opt_u32(args, CHAR_TYPE_NAME, span)?),
            DECIMAL_TYPE_NAME => CockroachType::Decimal(parse_two_opt_u32(args, DECIMAL_TYPE_NAME, span)?),
            FLOAT4_TYPE_NAME => CockroachType::Float4,
            FLOAT8_TYPE_NAME => CockroachType::Float8,
            INET_TYPE_NAME => CockroachType::Inet,
            INT2_TYPE_NAME => CockroachType::Int2,
            INT4_TYPE_NAME => CockroachType::Int4,
            INT8_TYPE_NAME => CockroachType::Int8,
            SINGLE_CHAR_TYPE_NAME => CockroachType::SingleChar,
            STRING_TYPE_NAME => CockroachType::String(parse_one_opt_u32(args, STRING_TYPE_NAME, span)?),
            TIMESTAMP_TYPE_NAME => CockroachType::Timestamp(parse_one_opt_u32(args, TIMESTAMP_TYPE_NAME, span)?),
            TIMESTAMP_TZ_TYPE_NAME => {
                CockroachType::Timestamptz(parse_one_opt_u32(args, TIMESTAMP_TZ_TYPE_NAME, span)?)
            }
            BIT_TYPE_NAME => CockroachType::Bit(parse_one_opt_u32(args, BIT_TYPE_NAME, span)?),
            BOOL_TYPE_NAME => CockroachType::Bool,
            DATE_TYPE_NAME => CockroachType::Date,
            JSON_B_TYPE_NAME => CockroachType::JsonB,
            TIME_TYPE_NAME => CockroachType::Time(parse_one_opt_u32(args, TIME_TYPE_NAME, span)?),
            TIME_TZ_TYPE_NAME => CockroachType::Timetz(parse_one_opt_u32(args, TIME_TZ_TYPE_NAME, span)?),
            UUID_TYPE_NAME => CockroachType::Uuid,
            VAR_BIT_TYPE_NAME => CockroachType::VarBit(parse_one_opt_u32(args, VAR_BIT_TYPE_NAME, span)?),
            _ => return Err(DatamodelError::new_native_type_parser_error(name, span)),
        };

        Ok(NativeTypeInstance::new(name, cloned_args, native_type.to_json()))
    }

    fn introspect_native_type(&self, native_type: serde_json::Value) -> NativeTypeInstance {
        let native_type: CockroachType = serde_json::from_value(native_type).unwrap();
        let (constructor_name, args) = match native_type {
            CockroachType::Int2 => (INT2_TYPE_NAME, vec![]),
            CockroachType::Int4 => (INT4_TYPE_NAME, vec![]),
            CockroachType::Int8 => (INT8_TYPE_NAME, vec![]),
            CockroachType::Decimal(x) => (DECIMAL_TYPE_NAME, args_vec_from_opt(x)),
            CockroachType::Float4 => (FLOAT4_TYPE_NAME, vec![]),
            CockroachType::Float8 => (FLOAT8_TYPE_NAME, vec![]),
            CockroachType::String(x) => (STRING_TYPE_NAME, arg_vec_from_opt(x)),
            CockroachType::Char(x) => (CHAR_TYPE_NAME, arg_vec_from_opt(x)),
            CockroachType::SingleChar => (SINGLE_CHAR_TYPE_NAME, Vec::new()),
            CockroachType::Bytes => (BYTES_TYPE_NAME, vec![]),
            CockroachType::Timestamp(x) => (TIMESTAMP_TYPE_NAME, arg_vec_from_opt(x)),
            CockroachType::Timestamptz(x) => (TIMESTAMP_TZ_TYPE_NAME, arg_vec_from_opt(x)),
            CockroachType::Date => (DATE_TYPE_NAME, vec![]),
            CockroachType::Time(x) => (TIME_TYPE_NAME, arg_vec_from_opt(x)),
            CockroachType::Timetz(x) => (TIME_TZ_TYPE_NAME, arg_vec_from_opt(x)),
            CockroachType::Bool => (BOOL_TYPE_NAME, vec![]),
            CockroachType::Bit(x) => (BIT_TYPE_NAME, arg_vec_from_opt(x)),
            CockroachType::VarBit(x) => (VAR_BIT_TYPE_NAME, arg_vec_from_opt(x)),
            CockroachType::Uuid => (UUID_TYPE_NAME, vec![]),
            CockroachType::JsonB => (JSON_B_TYPE_NAME, vec![]),
            CockroachType::Inet => (INET_TYPE_NAME, vec![]),
        };

        if let Some(constructor) = self.find_native_type_constructor(constructor_name) {
            NativeTypeInstance::new(constructor.name, args, native_type.to_json())
        } else {
            unreachable!()
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
            return Err("must start with the protocol `postgresql://` or `postgres://`.".to_owned());
        }

        Ok(())
    }
}
