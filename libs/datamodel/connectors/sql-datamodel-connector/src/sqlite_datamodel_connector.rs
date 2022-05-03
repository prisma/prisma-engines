use datamodel_connector::{
    parser_database::ScalarType, Connector, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance,
    ReferentialAction, ReferentialIntegrity,
};
use datamodel_connector::{ConstraintScope, DatamodelError, Span};
use enumflags2::BitFlags;
use std::borrow::Cow;

const NATIVE_TYPE_CONSTRUCTORS: &[NativeTypeConstructor] = &[];
const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::GlobalKeyIndex];
const CAPABILITIES: &[ConnectorCapability] = &[
    ConnectorCapability::AnyId,
    ConnectorCapability::AutoIncrement,
    ConnectorCapability::CompoundIds,
    ConnectorCapability::SqlQueryRaw,
    ConnectorCapability::RelationFieldsInArbitraryOrder,
    ConnectorCapability::UpdateableId,
    ConnectorCapability::ImplicitManyToManyRelation,
    ConnectorCapability::DecimalType,
    ConnectorCapability::BackwardCompatibleQueryRaw,
];

pub struct SqliteDatamodelConnector;

impl Connector for SqliteDatamodelConnector {
    fn name(&self) -> &str {
        "sqlite"
    }

    fn capabilities(&self) -> &'static [ConnectorCapability] {
        CAPABILITIES
    }

    fn max_identifier_length(&self) -> usize {
        10000
    }

    fn referential_actions(&self, referential_integrity: &ReferentialIntegrity) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        referential_integrity.allowed_referential_actions(SetNull | SetDefault | Cascade | Restrict | NoAction)
    }

    fn scalar_type_for_native_type(&self, _native_type: serde_json::Value) -> ScalarType {
        unreachable!("No native types on Sqlite");
    }

    fn default_native_type_for_scalar_type(&self, _scalar_type: &ScalarType) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        _native_type: serde_json::Value,
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

    fn parse_native_type(
        &self,
        _name: &str,
        _args: Vec<String>,
        span: Span,
    ) -> Result<NativeTypeInstance, DatamodelError> {
        Err(DatamodelError::new_native_types_not_supported(
            self.name().to_owned(),
            span,
        ))
    }

    fn introspect_native_type(&self, _native_type: serde_json::Value) -> NativeTypeInstance {
        unreachable!("unreachable introspect_native_type() on sqlite")
    }

    fn set_config_dir<'a>(&self, config_dir: &std::path::Path, url: &'a str) -> Cow<'a, str> {
        let set_root = |path: &str| {
            let path = std::path::Path::new(path);

            if path.is_relative() {
                Some(config_dir.join(&path).to_str().map(ToString::to_string).unwrap())
            } else {
                None
            }
        };

        if let Some(path) = set_root(url.trim_start_matches("file:").trim_start_matches("sqlite:")) {
            return Cow::Owned(format!("file:{}", path));
        };

        Cow::Borrowed(url)
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("file") && !url.starts_with("sqlite:") {
            return Err("must start with the protocol `file:`.".to_string());
        }

        Ok(())
    }
}
