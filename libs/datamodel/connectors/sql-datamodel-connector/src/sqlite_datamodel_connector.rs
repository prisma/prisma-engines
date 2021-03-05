use datamodel_connector::{connector_error::ConnectorError, Connector, ConnectorCapability};
use dml::model::Model;
use dml::native_type_constructor::NativeTypeConstructor;
use dml::native_type_instance::NativeTypeInstance;
use dml::{field::Field, scalars::ScalarType};

pub struct SqliteDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl SqliteDatamodelConnector {
    pub fn new() -> SqliteDatamodelConnector {
        let capabilities = vec![
            ConnectorCapability::RelationFieldsInArbitraryOrder,
            ConnectorCapability::UpdateableId,
        ];

        let constructors: Vec<NativeTypeConstructor> = vec![];

        SqliteDatamodelConnector {
            capabilities,
            constructors,
        }
    }
}

impl Connector for SqliteDatamodelConnector {
    fn name(&self) -> String {
        "sqlite".to_string()
    }
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
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
}

impl Default for SqliteDatamodelConnector {
    fn default() -> Self {
        Self::new()
    }
}
