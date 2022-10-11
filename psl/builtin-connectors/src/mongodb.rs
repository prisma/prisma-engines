mod mongodb_types;
mod validations;

use enumflags2::BitFlags;
use mongodb_types::*;
use native_types::{MongoDbType, NativeType};
use psl_core::{
    datamodel_connector::{
        Connector, ConnectorCapability, ConstraintScope, NativeTypeConstructor, NativeTypeInstance, RelationMode,
    },
    diagnostics::{DatamodelError, Diagnostics, Span},
    parser_database::{walkers::*, ReferentialAction, ScalarType},
};
use std::result::Result as StdResult;

const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::Json,
    ConnectorCapability::Enums,
    ConnectorCapability::EnumArrayPush,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::CreateMany,
    ConnectorCapability::ScalarLists,
    ConnectorCapability::JsonLists,
    ConnectorCapability::InsensitiveFilters,
    ConnectorCapability::CompositeTypes,
    ConnectorCapability::FullTextIndex,
    ConnectorCapability::SortOrderInFullTextIndex,
    ConnectorCapability::MongoDbQueryRaw,
    ConnectorCapability::DefaultValueAuto,
    ConnectorCapability::TwoWayEmbeddedManyToManyRelation,
    ConnectorCapability::UndefinedType,
];

pub(crate) struct MongoDbDatamodelConnector;

impl Connector for MongoDbDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "mongodb"
    }

    fn name(&self) -> &str {
        "MongoDB"
    }

    fn capabilities(&self) -> &'static [ConnectorCapability] {
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

    fn emulated_referential_actions(&self, relation_mode: &RelationMode) -> BitFlags<ReferentialAction> {
        relation_mode.allowed_emulated_referential_actions_default()
    }

    fn validate_model(&self, model: ModelWalker<'_>, errors: &mut Diagnostics) {
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

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        NATIVE_TYPE_CONSTRUCTORS
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let native_type = default_for(scalar_type);
        serde_json::to_value(native_type).expect("MongoDB native type to JSON failed")
    }

    fn native_type_is_default_for_scalar_type(&self, native_type: serde_json::Value, scalar_type: &ScalarType) -> bool {
        let default_native_type = default_for(scalar_type);
        let native_type: MongoDbType =
            serde_json::from_value(native_type).expect("MongoDB native type from JSON failed");

        &native_type == default_native_type
    }

    fn parse_native_type(
        &self,
        name: &str,
        args: Vec<String>,
        span: Span,
    ) -> Result<NativeTypeInstance, DatamodelError> {
        let mongo_type = mongo_type_from_input(name, span)?;

        Ok(NativeTypeInstance::new(name, args, mongo_type.to_json()))
    }

    fn introspect_native_type(&self, _native_type: serde_json::Value) -> NativeTypeInstance {
        // Out of scope for MVP
        todo!()
    }

    fn scalar_type_for_native_type(&self, _native_type: serde_json::Value) -> ScalarType {
        // Out of scope for MVP
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
}
