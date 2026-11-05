mod native_types;

use std::borrow::Cow;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
pub use native_types::KingbaseMySqlType;
use parser_database::{ExtensionTypes, ScalarFieldType};
use prisma_value::{PrismaValueResult, decode_bytes};

use crate::{
    ValidatedSchema,
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, Flavour, JoinStrategySupport,
        NativeTypeConstructor, NativeTypeInstance,
    },
    diagnostics::{Diagnostics, Span},
    parser_database::{ReferentialAction, ScalarType},
};
use KingbaseMySqlType::*;
use enumflags2::BitFlags;

pub const CAPABILITIES: ConnectorCapabilities = enumflags2::make_bitflags!(ConnectorCapability::{
    Enums |
    EnumArrayPush |
    Json |
    AutoIncrementAllowedOnNonId |
    RelationFieldsInArbitraryOrder |
    CreateMany |
    WritableAutoincField |
    CreateSkipDuplicates |
    UpdateableId |
    JsonFiltering |
    JsonFilteringJsonPath |
    JsonFilteringAlphanumeric |
    JsonArrayContains |
    CreateManyWriteableAutoIncId |
    AutoIncrement |
    CompoundIds |
    AnyId |
    NamedForeignKeys |
    AdvancedJsonNullability |
    FullTextIndex |
    NativeFullTextSearch |
    NativeFullTextSearchWithIndex |
    MultipleFullTextAttributesPerModel |
    ImplicitManyToManyRelation |
    DecimalType |
    OrderByNullsFirstLast |
    FilteredInlineChildNestedToOneDisconnect |
    SupportsTxIsolationReadUncommitted |
    SupportsTxIsolationReadCommitted |
    SupportsTxIsolationRepeatableRead |
    SupportsTxIsolationSerializable |
    RowIn |
    SupportsFiltersOnRelationsWithoutJoins |
    CorrelatedSubqueries |
    SupportsDefaultInInsert
});

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::GlobalForeignKey, ConstraintScope::ModelKeyIndex];
const DATE_TIME_DEFAULT: KingbaseMySqlType = KingbaseMySqlType::DateTime(Some(3));
const SCALAR_TYPE_DEFAULTS: &[(ScalarType, KingbaseMySqlType)] = &[
    (ScalarType::Int, KingbaseMySqlType::Int),
    (ScalarType::BigInt, KingbaseMySqlType::BigInt),
    (ScalarType::Float, KingbaseMySqlType::Double),
    (ScalarType::Decimal, KingbaseMySqlType::Decimal(Some((65, 30)))),
    (ScalarType::Boolean, KingbaseMySqlType::TinyInt),
    (ScalarType::String, KingbaseMySqlType::VarChar(191)),
    (ScalarType::DateTime, DATE_TIME_DEFAULT),
    (ScalarType::Bytes, KingbaseMySqlType::LongBlob),
    (ScalarType::Json, KingbaseMySqlType::Json),
];

pub struct KingbaseMysqlDatamodelConnector;

impl Connector for KingbaseMysqlDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "kingbase-mysql"
    }

    fn name(&self) -> &str {
        "Kingbase MySQL"
    }

    fn is_provider(&self, name: &str) -> bool {
        name == "kingbase-mysql"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        // Kingbase's max_identifier_length defaults to 63 bytes, including
        // in the tested MySQL mode. See the V9R3C11 FAQ, parameter section 3.6:
        // https://help.kingbase.com.cn/v9.3.11/faq/faq-new/parameter.html
        63
    }

    fn foreign_key_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        // V9R3C11 MySQL-mode CREATE TABLE documents SET DEFAULT for both
        // ON DELETE and ON UPDATE. Emulated relations use their own action set.
        Restrict | Cascade | SetNull | NoAction | SetDefault
    }

    fn scalar_type_for_native_type(
        &self,
        native_type: &NativeTypeInstance,
        _extension_types: &dyn ExtensionTypes,
    ) -> Option<ScalarFieldType> {
        let native_type: &KingbaseMySqlType = native_type.downcast_ref();
        let scalar_type = match native_type {
            VarChar(_) | Text | Char(_) | TinyText | MediumText | LongText => ScalarType::String,
            Int | SmallInt | MediumInt | Year | TinyInt | UnsignedInt | UnsignedSmallInt | UnsignedTinyInt
            | UnsignedMediumInt => ScalarType::Int,
            BigInt | UnsignedBigInt => ScalarType::BigInt,
            Float | Double => ScalarType::Float,
            Decimal(_) => ScalarType::Decimal,
            DateTime(_) | Date | Time(_) | Timestamp(_) => ScalarType::DateTime,
            Json => ScalarType::Json,
            LongBlob | Binary(_) | VarBinary(_) | TinyBlob | Blob | MediumBlob | Bit(_) => ScalarType::Bytes,
        };

        Some(ScalarFieldType::BuiltInScalar(scalar_type))
    }

    fn default_native_type_for_scalar_type(
        &self,
        scalar_type: &ScalarFieldType,
        _schema: &ValidatedSchema,
    ) -> Option<NativeTypeInstance> {
        let scalar_type = scalar_type.as_builtin_scalar()?;
        let native_type = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(candidate, _)| candidate == &scalar_type)
            .map(|(_, native_type)| *native_type)?;

        Some(NativeTypeInstance::new::<KingbaseMySqlType>(native_type))
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        scalar_type: Option<ScalarType>,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: &KingbaseMySqlType = native_type_instance.downcast_ref();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            UnsignedTinyInt | UnsignedSmallInt | UnsignedMediumInt => {
                errors.push_error(crate::diagnostics::DatamodelError::new_validation_error(
                    "Small unsigned integer native types are not supported by this Kingbase MySQL connector. Use a supported native type or migrate the column explicitly; Prisma does not emulate these types with domains or triggers.", span,
                ));
            }
            // V9R3C11 MySQL SQL reference, data types 2.2.8 (DECIMAL).
            Decimal(Some((precision, _))) if !(1..=1000).contains(precision) => {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision can range from 1 to 1000.", span))
            }
            Decimal(Some((_, scale))) if *scale > 1000 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Scale can range from 0 to 1000.", span))
            }
            Bit(length) if *length == 0 || *length > 64 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 1 to 64.", span))
            }
            // V9R3C11 MySQL SQL reference, data types 2.4.3/2.4.14.
            // C010 also accepts zero-length CHAR/VARCHAR; keep that existing support.
            Char(length) | VarChar(length) if *length > 10_485_760 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 10,485,760.", span))
            }
            Bit(n) if *n > 1 && matches!(scalar_type, Some(ScalarType::Boolean)) => {
                errors.push_error(error.new_argument_m_out_of_range_error("only Bit(1) can be used as Boolean.", span))
            }
            _ => (),
        }
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        CONSTRAINT_SCOPES
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        native_types::CONSTRUCTORS
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: &[String],
        span: Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance> {
        match KingbaseMySqlType::from_parts(name, args) {
            // The server raises precision to scale when scale is larger. Normalize
            // valid arguments here so schema calculation and query compilation use
            // the same effective type as introspection, avoiding repeated ALTERs.
            // Preserve invalid arguments for validate_native_type_arguments().
            Ok(Decimal(Some((precision, scale)))) if (1..=1000).contains(&precision) && scale <= 1000 => {
                Some(NativeTypeInstance::new(Decimal(Some((precision.max(scale), scale)))))
            }
            Ok(native_type) => Some(NativeTypeInstance::new(native_type)),
            Err(error) => {
                diagnostics.push_error(error.into_datamodel_error(span));
                None
            }
        }
    }

    fn native_type_to_parts<'t>(&self, native_type: &'t NativeTypeInstance) -> (&'t str, Cow<'t, [String]>) {
        native_type.downcast_ref::<KingbaseMySqlType>().to_parts()
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("kingbase-mysql://") && !url.starts_with("kingbase://") {
            return Err("must start with the protocol `kingbase-mysql://` or `kingbase://`.".to_owned());
        }

        Ok(())
    }

    fn flavour(&self) -> Flavour {
        Flavour::KingbaseMysql
    }

    fn parse_json_datetime(
        &self,
        value: &str,
        native_type: Option<NativeTypeInstance>,
    ) -> chrono::ParseResult<DateTime<FixedOffset>> {
        match native_type
            .as_ref()
            .map(|native_type| native_type.downcast_ref::<KingbaseMySqlType>())
        {
            Some(Date) => parse_date(value),
            Some(Time(_)) => parse_time(value),
            Some(DateTime(_) | Timestamp(_)) | None => parse_datetime(value),
            _ => unreachable!(),
        }
    }

    fn parse_json_bytes(&self, value: &str, _native_type: Option<NativeTypeInstance>) -> PrismaValueResult<Vec<u8>> {
        let mut buffer = vec![0; value.len()];
        decode_bytes(sanitize_base64(value, &mut buffer))
    }

    fn runtime_join_strategy_support(&self) -> JoinStrategySupport {
        if self.static_join_strategy_support() {
            JoinStrategySupport::UnknownYet
        } else {
            JoinStrategySupport::No
        }
    }

    fn supports_shard_keys(&self) -> bool {
        true
    }
}

fn parse_date(value: &str) -> chrono::ParseResult<DateTime<FixedOffset>> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|date| DateTime::<Utc>::from_naive_utc_and_offset(date.and_hms_opt(0, 0, 0).unwrap(), Utc))
        .map(DateTime::<FixedOffset>::from)
}

fn parse_time(value: &str) -> chrono::ParseResult<DateTime<FixedOffset>> {
    NaiveTime::parse_from_str(value, "%H:%M:%S%.f")
        .map(|time| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().and_time(time))
        .map(|datetime| DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc))
        .map(DateTime::<FixedOffset>::from)
}

fn parse_datetime(value: &str) -> chrono::ParseResult<DateTime<FixedOffset>> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f")
        .map(|datetime| DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc))
        .or_else(|_| DateTime::parse_from_rfc3339(value).map(DateTime::<Utc>::from))
        .map(DateTime::<FixedOffset>::from)
}

fn sanitize_base64<'a>(mut value: &str, buffer: &'a mut [u8]) -> &'a [u8] {
    let mut position = 0;

    while !value.is_empty() && position < buffer.len() {
        let newline = value.find('\n').unwrap_or(value.len());
        let length = newline.min(buffer.len() - position);
        buffer[position..position + length].copy_from_slice(&value.as_bytes()[..length]);
        position += length;
        value = &value[(newline + 1).min(value.len())..];
    }

    &buffer[..position]
}
