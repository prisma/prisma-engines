mod native_types;
mod validations;

use std::borrow::Cow;

use enumflags2::{make_bitflags, BitFlags};
use parser_database::{ExtensionTypes, ScalarFieldType};

pub use native_types::{KingbaseOracleNumberArguments, KingbaseOracleType};

use crate::{
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, Flavour, NativeTypeConstructor,
        NativeTypeInstance, RelationMode, StringFilter,
    },
    diagnostics::{Diagnostics, Span},
    parser_database::{walkers, ReferentialAction, ScalarType},
    ValidatedSchema,
};

use KingbaseOracleNumberArguments::*;
use KingbaseOracleType::*;

pub const CAPABILITIES: ConnectorCapabilities = make_bitflags!(ConnectorCapability::{
    Enums |
    Json |
    AdvancedJsonNullability |
    JsonFiltering |
    JsonFilteringArrayPath |
    JsonFilteringAlphanumeric |
    JsonFilteringAlphanumericFieldRef |
    JsonArrayContains |
    InsensitiveFilters |
    LateralJoin |
    NativeFullTextSearch |
    NativeFullTextSearchWithoutIndex |
    AutoIncrement |
    AutoIncrementAllowedOnNonId |
    RelationFieldsInArbitraryOrder |
    CreateMany |
    InsertReturning |
    UpdateReturning |
    DeleteReturning |
    WritableAutoincField |
    CreateManyWriteableAutoIncId |
    UpdateableId |
    CompoundIds |
    AnyId |
    NamedForeignKeys |
    NamedPrimaryKeys |
    MultiSchema |
    ImplicitManyToManyRelation |
    DecimalType |
    OrderByNullsFirstLast |
    FilteredInlineChildNestedToOneDisconnect |
    SupportsTxIsolationReadCommitted |
    SupportsTxIsolationSerializable |
    RowIn |
    SupportsFiltersOnRelationsWithoutJoins |
    SupportsDefaultInInsert
});

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[
    ConstraintScope::GlobalPrimaryKeyKeyIndex,
    ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey,
];

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, KingbaseOracleType)] = &[
    (ScalarType::Int, Number(PrecisionAndScale(10, 0))),
    (ScalarType::BigInt, Number(PrecisionAndScale(19, 0))),
    (ScalarType::Float, BinaryDouble),
    (ScalarType::Decimal, Number(Unspecified)),
    (ScalarType::Boolean, Boolean),
    (ScalarType::String, VarChar2(Some(4000))),
    (ScalarType::DateTime, Timestamp(Some(3))),
    (ScalarType::Bytes, Blob),
    (ScalarType::Json, Json),
];

pub struct KingbaseOracleDatamodelConnector;

impl Connector for KingbaseOracleDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "kingbase-oracle"
    }

    fn name(&self) -> &str {
        "Kingbase Oracle"
    }

    fn is_provider(&self, name: &str) -> bool {
        name == "kingbase-oracle"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        63
    }

    fn foreign_key_referential_actions(&self) -> BitFlags<ReferentialAction> {
        ReferentialAction::NoAction
            | ReferentialAction::Restrict
            | ReferentialAction::Cascade
            | ReferentialAction::SetNull
            | ReferentialAction::SetDefault
    }

    fn scalar_type_for_native_type(
        &self,
        native_type: &NativeTypeInstance,
        _extension_types: &dyn ExtensionTypes,
    ) -> Option<ScalarFieldType> {
        let native_type: &KingbaseOracleType = native_type.downcast_ref();
        let scalar_type = match native_type {
            TinyInt | BinaryInteger => ScalarType::Int,
            Number(Precision(precision) | PrecisionAndScale(precision, 0)) if *precision <= 10 => ScalarType::Int,
            Number(Precision(precision) | PrecisionAndScale(precision, 0)) if *precision <= 19 => ScalarType::BigInt,
            Number(_) => ScalarType::Decimal,
            Float(_) | BinaryFloat | BinaryDouble => ScalarType::Float,
            Char(_) | VarChar2(_) | CharByte(_) | VarChar2Byte(_) | RowId | URowId(_) | NChar(_) | NVarChar2(_)
            | Clob | NClob | Uuid | Xml => ScalarType::String,
            Blob => ScalarType::Bytes,
            Date | Timestamp(_) | TimestampTz(_) | TimestampLocalTz(_) => ScalarType::DateTime,
            Boolean => ScalarType::Boolean,
            Json => ScalarType::Json,
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

        Some(NativeTypeInstance::new::<KingbaseOracleType>(native_type))
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        _scalar_type: Option<ScalarType>,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: &KingbaseOracleType = native_type_instance.downcast_ref();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            Number(Precision(0) | PrecisionAndScale(0, _)) => {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision must be between 1 and 38.", span))
            }
            Number(Precision(precision) | PrecisionAndScale(precision, _)) if *precision > 38 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision must be between 1 and 38.", span))
            }
            Number(PrecisionAndScale(_, scale)) if *scale > 1000 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Scale must be between 0 and 1000.", span))
            }
            Float(Some(precision)) if *precision == 0 || *precision > 53 => {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision must be between 1 and 53.", span))
            }
            VarChar2(Some(length))
            | Char(Some(length))
            | NChar(Some(length))
            | NVarChar2(Some(length))
            | CharByte(length)
            | VarChar2Byte(length)
            | URowId(Some(length))
                if *length == 0 || *length > 10_485_760 =>
            {
                errors.push_error(
                    error.new_argument_m_out_of_range_error("Length must be between 1 and 10,485,760.", span),
                )
            }
            Timestamp(Some(precision)) | TimestampTz(Some(precision)) | TimestampLocalTz(Some(precision))
                if *precision > 6 =>
            {
                errors.push_error(error.new_argument_m_out_of_range_error("Precision must be between 0 and 6.", span))
            }
            _ => (),
        }
    }

    fn validate_model(&self, model: walkers::ModelWalker<'_>, relation_mode: RelationMode, errors: &mut Diagnostics) {
        if relation_mode.uses_foreign_keys() {
            validations::foreign_keys_must_reference_constraints(model, errors);
        }

        for index in model.indexes() {
            validations::field_types_can_be_used_in_an_index(self, index, errors);
        }

        if let Some(primary_key) = model.primary_key() {
            validations::field_types_can_be_used_in_a_primary_key(self, primary_key, errors);
        }
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        CONSTRAINT_SCOPES
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        &native_types::CONSTRUCTORS_WITH_ALIASES
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: &[String],
        span: Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance> {
        let name = if name == "Decimal" { "Number" } else { name };
        if name == "Number" && args.get(1).is_some_and(|scale| scale.starts_with('-')) {
            let native_type = NativeTypeInstance::new(Number(Unspecified));
            diagnostics.push_error(self.native_instance_error(&native_type).new_argument_m_out_of_range_error(
                "Negative NUMBER scale is not supported: this Kingbase Oracle version does not preserve Oracle rounding semantics.", span,
            ));
            return None;
        }
        match KingbaseOracleType::from_parts(name, args) {
            // R2 raises catalog precision to scale. Use a legal representative
            // for that physical type without exceeding Oracle declaration precision.
            // Introspection uses the same representation to avoid repeated ALTERs.
            // Keep invalid arguments intact for the normal bounds diagnostics.
            Ok(Number(PrecisionAndScale(precision, scale))) if (1..=38).contains(&precision) && scale <= 1000 => Some(
                NativeTypeInstance::new(Number(PrecisionAndScale(precision.max(scale).min(38), scale))),
            ),
            Ok(native_type) => Some(NativeTypeInstance::new(native_type)),
            Err(error) => {
                diagnostics.push_error(error.into_datamodel_error(span));
                None
            }
        }
    }

    fn native_type_to_parts<'t>(&self, native_type: &'t NativeTypeInstance) -> (&'t str, Cow<'t, [String]>) {
        native_type.downcast_ref::<KingbaseOracleType>().to_parts()
    }

    fn scalar_filter_name(&self, scalar_type_name: String, native_type_name: Option<&str>) -> Cow<'_, str> {
        match native_type_name {
            Some(name) if name.eq_ignore_ascii_case("uuid") => "Uuid".into(),
            _ => scalar_type_name.into(),
        }
    }

    fn string_filters(&self, input_object_name: &str) -> BitFlags<StringFilter> {
        match input_object_name {
            "Uuid" => BitFlags::empty(),
            _ => BitFlags::all(),
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("kingbase-oracle://") {
            return Err("must start with the protocol `kingbase-oracle://`.".to_owned());
        }

        Ok(())
    }

    fn flavour(&self) -> Flavour {
        Flavour::KingbaseOracle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uuid_filters_are_distinct_from_strings_and_disable_string_operations() {
        let connector = KingbaseOracleDatamodelConnector;
        for native_name in ["Uuid", "uuid", "UUID"] {
            let name = connector.scalar_filter_name("String".to_owned(), Some(native_name));
            assert_eq!(name, "Uuid");
            assert!(connector.string_filters(&name).is_empty());
        }
        for native_name in [None, Some("VarChar2"), Some("NVarChar2"), Some("Clob")] {
            let name = connector.scalar_filter_name("String".to_owned(), native_name);
            assert_eq!(name, "String");
            assert_eq!(connector.string_filters(&name), BitFlags::all());
        }
    }
}
