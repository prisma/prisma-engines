mod mongodb_types;
mod validations;

pub use mongodb_types::MongoDbType;

use enumflags2::BitFlags;
use mongodb_types::*;
use psl_core::{
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, NativeTypeConstructor,
        NativeTypeInstance, RelationMode,
    },
    diagnostics::{Diagnostics, Span},
    parser_database::{walkers::*, ReferentialAction, ScalarType},
};
use std::result::Result as StdResult;

const CAPABILITIES: ConnectorCapabilities = enumflags2::make_bitflags!(ConnectorCapability::{
    Json |
    Enums |
    EnumArrayPush |
    RelationFieldsInArbitraryOrder |
    CreateMany |
    ScalarLists |
    JsonLists |
    InsensitiveFilters |
    CompositeTypes |
    FullTextIndex |
    SortOrderInFullTextIndex |
    MongoDbQueryRaw |
    DefaultValueAuto |
    TwoWayEmbeddedManyToManyRelation |
    UndefinedType
});

pub(crate) struct MongoDbDatamodelConnector;

impl Connector for MongoDbDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "mongodb"
    }

    fn name(&self) -> &str {
        "MongoDB"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        127
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        &[ConstraintScope::ModelKeyIndex]
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        BitFlags::empty()
    }

    fn validate_model(&self, model: ModelWalker<'_>, _: RelationMode, errors: &mut Diagnostics) {
        validations::id_must_be_defined(model, errors);

        if let Some(pk) = model.primary_key() {
            validations::id_field_must_have_a_correct_mapped_name(pk, errors);
        }

        for field in model.scalar_fields() {
            validations::objectid_type_required_with_auto_attribute(field, errors);
            validations::auto_attribute_must_be_an_id(field, errors);
            validations::dbgenerated_attribute_is_not_allowed(field, errors);
            validations::field_name_uses_valid_characters(field, errors);
        }

        for index in model.indexes() {
            validations::index_is_not_defined_multiple_times_to_same_fields(index, errors);
            validations::unique_cannot_be_defined_to_id_field(index, errors);
        }
    }

    fn validate_relation_field(
        &self,
        field: psl_core::parser_database::walkers::RelationFieldWalker<'_>,
        errors: &mut Diagnostics,
    ) {
        validations::relation_same_native_type(field, errors);
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        mongodb_types::CONSTRUCTORS
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> NativeTypeInstance {
        let native_type = default_for(scalar_type);
        NativeTypeInstance::new::<MongoDbType>(*native_type)
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: &NativeTypeInstance,
        scalar_type: &ScalarType,
    ) -> bool {
        let default_native_type = default_for(scalar_type);
        let native_type: &MongoDbType = native_type.downcast_ref();
        native_type == default_native_type
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: &[String],
        span: Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance> {
        let native_type = MongoDbType::from_parts(name, args, span, diagnostics)?;
        Some(NativeTypeInstance::new::<MongoDbType>(native_type))
    }

    fn native_type_to_parts(&self, native_type: &NativeTypeInstance) -> (&'static str, Vec<String>) {
        native_type.downcast_ref::<MongoDbType>().to_parts()
    }

    fn scalar_type_for_native_type(&self, _native_type: &NativeTypeInstance) -> ScalarType {
        todo!()
    }

    fn validate_url(&self, url: &str) -> StdResult<(), String> {
        if !url.starts_with("mongo") {
            return Err("must start with the protocol `mongo`.".into());
        }

        Ok(())
    }

    fn default_relation_mode(&self) -> RelationMode {
        RelationMode::Prisma
    }

    fn allowed_relation_mode_settings(&self) -> enumflags2::BitFlags<RelationMode> {
        RelationMode::Prisma.into()
    }

    /// Avoid checking whether the fields appearing in a `@relation` attribute are included in an index.
    fn should_suggest_missing_referencing_fields_indexes(&self) -> bool {
        false
    }
}
