use crate::{
    datamodel_connector::{
        Connector, ConnectorCapabilities, ConnectorCapability, ConstraintScope, Flavour, NativeTypeConstructor,
        NativeTypeInstance, ValidatedConnector,
    },
    diagnostics::{DatamodelError, Diagnostics, Span},
    parser_database::{ReferentialAction, ScalarType},
};
use enumflags2::BitFlags;

const NATIVE_TYPE_CONSTRUCTORS: &[NativeTypeConstructor] = &[];
const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::GlobalKeyIndex];
const CAPABILITIES: ConnectorCapabilities = enumflags2::make_bitflags!(ConnectorCapability::{
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
    SupportsFiltersOnRelationsWithoutJoins
});

pub struct SqliteDatamodelConnector;

impl ValidatedConnector for SqliteDatamodelConnector {
    fn provider_name(&self) -> &'static str {
        "sqlite"
    }

    fn name(&self) -> &str {
        "sqlite"
    }

    fn flavour(&self) -> Flavour {
        Flavour::Sqlite
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        CAPABILITIES
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        SetNull | SetDefault | Cascade | Restrict | NoAction
    }

    fn emulated_referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        Restrict | SetNull | Cascade
    }

    fn default_native_type_for_scalar_type(&self, _scalar_type: &ScalarType) -> Option<NativeTypeInstance> {
        None
    }

    fn native_type_to_parts(&self, _native_type: &NativeTypeInstance) -> (&'static str, Vec<String>) {
        unreachable!()
    }

    fn parse_native_type(
        &self,
        _name: &str,
        _args: &[String],
        span: Span,
        diagnostics: &mut Diagnostics,
    ) -> Option<NativeTypeInstance> {
        diagnostics.push_error(DatamodelError::new_native_types_not_supported(
            self.name().to_owned(),
            span,
        ));
        None
    }
}

impl Connector for SqliteDatamodelConnector {
    fn max_identifier_length(&self) -> usize {
        10000
    }

    fn scalar_type_for_native_type(&self, _native_type: &NativeTypeInstance) -> ScalarType {
        unreachable!("No native types on Sqlite");
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        _native_type: &NativeTypeInstance,
        _scalar_type: &ScalarType,
    ) -> bool {
        false
    }

    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        CONSTRAINT_SCOPES
    }

    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor] {
        NATIVE_TYPE_CONSTRUCTORS
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("file") {
            return Err("must start with the protocol `file:`.".to_string());
        }

        Ok(())
    }
}
