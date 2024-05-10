mod native_types;
mod validations;

use chrono::FixedOffset;
pub use native_types::MySqlType;
use prisma_value::{decode_bytes, PrismaValueResult};

use crate::{
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, Flavour, JoinStrategySupport,
        NativeTypeConstructor, NativeTypeInstance, RelationMode,
    },
    diagnostics::{Diagnostics, Span},
    parser_database::{walkers, ReferentialAction, ScalarType},
};
use enumflags2::BitFlags;
use MySqlType::*;

const TINY_BLOB_TYPE_NAME: &str = "TinyBlob";
const BLOB_TYPE_NAME: &str = "Blob";
const MEDIUM_BLOB_TYPE_NAME: &str = "MediumBlob";
const LONG_BLOB_TYPE_NAME: &str = "LongBlob";
const TINY_TEXT_TYPE_NAME: &str = "TinyText";
const TEXT_TYPE_NAME: &str = "Text";
const MEDIUM_TEXT_TYPE_NAME: &str = "MediumText";
const LONG_TEXT_TYPE_NAME: &str = "LongText";

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
    CreateManyWriteableAutoIncId |
    AutoIncrement |
    CompoundIds |
    AnyId |
    SqlQueryRaw |
    NamedForeignKeys |
    AdvancedJsonNullability |
    IndexColumnLengthPrefixing |
    FullTextIndex |
    FullTextSearch |
    FullTextSearchWithIndex |
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

    fn is_provider(&self, name: &str) -> bool {
        name == "mysql"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        64
    }

    fn foreign_key_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        Restrict | Cascade | SetNull | NoAction | SetDefault
    }

    fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> ScalarType {
        let native_type: &MySqlType = native_type.downcast_ref();

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

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> Option<NativeTypeInstance> {
        let native_type = SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .ok_or_else(|| format!("Could not find scalar type {scalar_type:?} in SCALAR_TYPE_DEFAULTS"))
            .unwrap();

        Some(NativeTypeInstance::new::<MySqlType>(*native_type))
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: &NativeTypeInstance,
        scalar_type: &ScalarType,
    ) -> bool {
        let native_type: &MySqlType = native_type.downcast_ref();

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && native_type == nt)
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        scalar_type: &ScalarType,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: &MySqlType = native_type_instance.downcast_ref();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            Decimal(Some((precision, scale))) if scale > precision => {
                errors.push_error(error.new_scale_larger_than_precision_error(span))
            }
            Decimal(Some((precision, _))) if *precision > 65 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision can range from 1 to 65.", span))
            }
            Decimal(Some((_, scale))) if *scale > 30 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Scale can range from 0 to 30.", span))
            }
            Bit(length) if *length == 0 || *length > 64 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 1 to 64.", span))
            }
            Char(length) if *length > 255 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 255.", span))
            }
            VarChar(length) if *length > 65535 => {
                errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 65,535.", span))
            }
            Bit(n) if *n > 1 && matches!(scalar_type, ScalarType::Boolean) => {
                errors.push_error(error.new_argument_m_out_of_range_error("only Bit(1) can be used as Boolean.", span))
            }
            _ => (),
        }
    }

    fn validate_model(&self, model: walkers::ModelWalker<'_>, relation_mode: RelationMode, errors: &mut Diagnostics) {
        for index in model.indexes() {
            validations::field_types_can_be_used_in_an_index(self, index, errors);
        }

        if let Some(pk) = model.primary_key() {
            validations::field_types_can_be_used_in_a_primary_key(self, pk, errors);
        }

        if relation_mode.uses_foreign_keys() {
            for field in model.relation_fields() {
                validations::uses_native_referential_action_set_default(self, field, errors);
            }
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
        let native_type = MySqlType::from_parts(name, args, span, diagnostics)?;
        Some(NativeTypeInstance::new::<MySqlType>(native_type))
    }

    fn native_type_to_parts(&self, native_type: &NativeTypeInstance) -> (&'static str, Vec<String>) {
        native_type.downcast_ref::<MySqlType>().to_parts()
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("mysql") {
            return Err("must start with the protocol `mysql://`.".to_owned());
        }

        Ok(())
    }

    fn flavour(&self) -> Flavour {
        Flavour::Mysql
    }

    fn parse_json_datetime(
        &self,
        str: &str,
        nt: Option<NativeTypeInstance>,
    ) -> chrono::ParseResult<chrono::DateTime<FixedOffset>> {
        let native_type: Option<&MySqlType> = nt.as_ref().map(|nt| nt.downcast_ref());

        match native_type {
            Some(pt) => match pt {
                Date => super::utils::common::parse_date(str),
                Time(_) => super::utils::common::parse_time(str),
                DateTime(_) => super::utils::mysql::parse_datetime(str),
                Timestamp(_) => super::utils::mysql::parse_timestamp(str),
                _ => unreachable!(),
            },
            None => self.parse_json_datetime(str, self.default_native_type_for_scalar_type(&ScalarType::DateTime)),
        }
    }

    // On MySQL, bytes are encoded as base64 in the database directly.
    fn parse_json_bytes(&self, str: &str, _nt: Option<NativeTypeInstance>) -> PrismaValueResult<Vec<u8>> {
        decode_bytes(str)
    }

    fn runtime_join_strategy_support(&self) -> JoinStrategySupport {
        match self.static_join_strategy_support() {
            // Prior to MySQL 8.0.14 and for MariaDB, a derived table cannot contain outer references.
            // Source: https://dev.mysql.com/doc/refman/8.0/en/derived-tables.html.
            true => JoinStrategySupport::UnknownYet,
            false => JoinStrategySupport::No,
        }
    }
}
