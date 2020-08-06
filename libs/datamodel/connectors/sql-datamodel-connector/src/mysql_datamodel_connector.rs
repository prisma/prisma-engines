use datamodel_connector::{Connector, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance};
use native_types::NativeType;

pub struct MySqlDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl MySqlDatamodelConnector {
    pub fn new() -> MySqlDatamodelConnector {
        let capabilities = vec![
            ConnectorCapability::RelationsOverNonUniqueCriteria,
            ConnectorCapability::Enums,
            ConnectorCapability::Json,
            ConnectorCapability::MultipleIndexesWithSameName,
        ];

        let constructors: Vec<NativeTypeConstructor> = vec![];

        MySqlDatamodelConnector {
            capabilities,
            constructors,
        }
    }
}

impl Connector for MySqlDatamodelConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, _name: &str, _args: Vec<u32>) -> Option<NativeTypeInstance> {
        None
    }

    fn introspect_native_type(&self, _native_type: Box<dyn NativeType>) -> Option<NativeTypeInstance> {
        None
    }
}
