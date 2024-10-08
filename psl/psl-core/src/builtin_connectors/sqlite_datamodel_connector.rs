mod native_types;
pub use native_types::SQLiteType;

use crate::{
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, Flavour, NativeTypeConstructor,
        NativeTypeInstance,
    },
    diagnostics::{Diagnostics, Span},
    parser_database::{ReferentialAction, ScalarType},
};
use enumflags2::BitFlags;

use super::geometry::GeometryParams;

const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::GlobalKeyIndex];
pub const CAPABILITIES: ConnectorCapabilities = enumflags2::make_bitflags!(ConnectorCapability::{
    AnyId |
    AutoIncrement |
    CompoundIds |
    SqlQueryRaw |
    RelationFieldsInArbitraryOrder |
    UpdateableId |
    ImplicitManyToManyRelation |
    DecimalType |
    BackwardCompatibleQueryRaw |
    OrderByNullsFirstLast |
    SupportsTxIsolationSerializable |
    NativeUpsert |
    FilteredInlineChildNestedToOneDisconnect |
    RowIn |
    InsertReturning |
    DeleteReturning |
    UpdateReturning |
    SupportsFiltersOnRelationsWithoutJoins |
    CreateMany |
    CreateManyWriteableAutoIncId
});

const SCALAR_TYPE_DEFAULTS: &[(ScalarType, SQLiteType)] =
    &[(ScalarType::Geometry, SQLiteType::Geometry(GeometryParams::default()))];

pub struct SqliteDatamodelConnector;

impl Connector for SqliteDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "sqlite"
    }

    fn name(&self) -> &str {
        "sqlite"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        10000
    }

    fn foreign_key_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        SetNull | SetDefault | Cascade | Restrict | NoAction
    }

    fn emulated_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        Restrict | SetNull | Cascade
    }

    fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> ScalarType {
        let native_type: &SQLiteType = native_type.downcast_ref();
        match native_type {
            SQLiteType::Geometry(_) => ScalarType::Geometry,
        }
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> Option<NativeTypeInstance> {
        SCALAR_TYPE_DEFAULTS
            .iter()
            .find(|(st, _)| st == scalar_type)
            .map(|(_, native_type)| native_type)
            .map(|nt| NativeTypeInstance::new::<SQLiteType>(*nt))
        // .unwrap_or(NativeTypeInstance::new(()))
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        native_type: &NativeTypeInstance,
        scalar_type: &ScalarType,
    ) -> bool {
        let native_type: &SQLiteType = native_type.downcast_ref();

        SCALAR_TYPE_DEFAULTS
            .iter()
            .any(|(st, nt)| scalar_type == st && native_type == nt)
    }

    fn validate_native_type_arguments(
        &self,
        native_type_instance: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        span: Span,
        errors: &mut Diagnostics,
    ) {
        let native_type: &SQLiteType = native_type_instance.downcast_ref();
        let error = self.native_instance_error(native_type_instance);

        match native_type {
            SQLiteType::Geometry(g) if g.srid < -1 => errors
                .push_error(error.new_argument_m_out_of_range_error("SRID must be superior or equal to -1.", span)),
            _ => (),
        }
    }

    fn native_type_to_parts(&self, native_type: &NativeTypeInstance) -> (&'static str, Vec<String>) {
        native_type.downcast_ref::<SQLiteType>().to_parts()
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
        SQLiteType::from_parts(name, args, span, diagnostics).map(NativeTypeInstance::new::<SQLiteType>)
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("file") {
            return Err("must start with the protocol `file:`.".to_string());
        }

        Ok(())
    }

    fn flavour(&self) -> Flavour {
        Flavour::Sqlite
    }
}
