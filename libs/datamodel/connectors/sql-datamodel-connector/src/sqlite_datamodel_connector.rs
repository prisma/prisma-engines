use datamodel_connector::ConstraintScope;
use datamodel_connector::{connector_error::ConnectorError, Connector, ConnectorCapability, ReferentialIntegrity};
use dml::{
    native_type_constructor::NativeTypeConstructor, native_type_instance::NativeTypeInstance,
    relation_info::ReferentialAction, scalars::ScalarType,
};
use enumflags2::BitFlags;
use std::borrow::Cow;

pub struct SqliteDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    referential_integrity: ReferentialIntegrity,
}

const NATIVE_TYPE_CONSTRUCTORS: &[NativeTypeConstructor] = &[];
const CONSTRAINT_SCOPES: &[ConstraintScope] = &[ConstraintScope::GlobalKeyIndex];

impl SqliteDatamodelConnector {
    pub fn new(referential_integrity: ReferentialIntegrity) -> SqliteDatamodelConnector {
        let mut capabilities = vec![
            ConnectorCapability::RelationFieldsInArbitraryOrder,
            ConnectorCapability::UpdateableId,
            ConnectorCapability::AutoIncrement,
            ConnectorCapability::CompoundIds,
            ConnectorCapability::AnyId,
            ConnectorCapability::QueryRaw,
        ];

        if referential_integrity.uses_foreign_keys() {
            capabilities.push(ConnectorCapability::ForeignKeys);
        }

        SqliteDatamodelConnector {
            capabilities,
            referential_integrity,
        }
    }
}

impl Connector for SqliteDatamodelConnector {
    fn name(&self) -> &str {
        "sqlite"
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn constraint_name_length(&self) -> usize {
        10000
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        self.referential_integrity
            .allowed_referential_actions(SetNull | SetDefault | Cascade | Restrict | NoAction)
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

    fn parse_native_type(&self, _name: &str, _args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError> {
        self.native_types_not_supported()
    }

    fn introspect_native_type(&self, _native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        self.native_types_not_supported()
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

        if let Some(path) = set_root(url.trim_start_matches("file:")) {
            return format!("file:{}", path).into();
        };

        url.into()
    }

    fn validate_url(&self, url: &str) -> Result<(), String> {
        if !url.starts_with("file") && !url.starts_with("sqlite:") {
            return Err("must start with the protocol `file:`.".to_string());
        }

        Ok(())
    }
}
