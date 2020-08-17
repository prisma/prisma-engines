use datamodel_connector::error::ConnectorError;
use datamodel_connector::{Connector, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance};
use native_types::NativeType;

pub struct MsSqlDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl MsSqlDatamodelConnector {
    pub fn new() -> MsSqlDatamodelConnector {
        let capabilities = vec![];
        let constructors: Vec<NativeTypeConstructor> = vec![];

        MsSqlDatamodelConnector {
            capabilities,
            constructors,
        }
    }
}

impl Connector for MsSqlDatamodelConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, _name: &str, _args: Vec<u32>) -> Result<NativeTypeInstance, ConnectorError> {
        return Err(ConnectorError::new_connector_not_supported_for_native_types("mssql"));
    }

    fn introspect_native_type(&self, _native_type: Box<dyn NativeType>) -> Result<NativeTypeInstance, ConnectorError> {
        return Err(ConnectorError::new_connector_not_supported_for_native_types("mssql"));
    }
}
