use datamodel_connector::{connector_error::ConnectorError, Connector, ConnectorCapability};
use dml::{
    field::Field, model::Model, native_type_constructor::NativeTypeConstructor,
    native_type_instance::NativeTypeInstance, relation_info::ReferentialAction, scalars::ScalarType,
};
use enumflags2::BitFlags;
use std::borrow::Cow;

pub struct SqliteDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
    referential_actions: BitFlags<ReferentialAction>,
}

impl SqliteDatamodelConnector {
    pub fn new() -> SqliteDatamodelConnector {
        use ReferentialAction::*;

        let capabilities = vec![
            ConnectorCapability::RelationFieldsInArbitraryOrder,
            ConnectorCapability::UpdateableId,
            ConnectorCapability::AutoIncrement,
            ConnectorCapability::CompoundIds,
            ConnectorCapability::ForeignKeys,
            ConnectorCapability::AnyId,
        ];

        let constructors: Vec<NativeTypeConstructor> = vec![];
        let referential_actions = SetNull | SetDefault | Cascade | Restrict | NoAction;

        SqliteDatamodelConnector {
            capabilities,
            constructors,
            referential_actions,
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
        self.referential_actions
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

    fn validate_field(&self, _field: &Field) -> Result<(), ConnectorError> {
        Ok(())
    }

    fn validate_model(&self, _model: &Model) -> Result<(), ConnectorError> {
        Ok(())
    }

    fn available_native_type_constructors(&self) -> &[NativeTypeConstructor] {
        &self.constructors
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

impl Default for SqliteDatamodelConnector {
    fn default() -> Self {
        Self::new()
    }
}
