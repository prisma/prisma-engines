use datamodel_connector::error::ConnectorError;
use datamodel_connector::scalars::ScalarType;
use datamodel_connector::{Connector, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance};
use native_types::NativeType;

pub struct SqliteDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl SqliteDatamodelConnector {
    pub fn new() -> SqliteDatamodelConnector {
        let capabilities = vec![];
        let constructors: Vec<NativeTypeConstructor> = vec![];

        SqliteDatamodelConnector {
            capabilities,
            constructors,
        }
    }
}

impl Connector for SqliteDatamodelConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(
        &self,
        _name: &str,
        _args: Vec<u32>,
        _scalar_type: ScalarType,
    ) -> Result<NativeTypeInstance, ConnectorError> {
        return Err(ConnectorError::new_connector_not_supported_for_native_types("sqlite"));
    }

    fn introspect_native_type(&self, _native_type: Box<dyn NativeType>) -> Result<NativeTypeInstance, ConnectorError> {
        return Err(ConnectorError::new_connector_not_supported_for_native_types("sqlite"));
    }
}
