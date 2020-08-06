use super::Connector;
use crate::{ConnectorCapability, NativeTypeConstructor, NativeTypeInstance};
use native_types::NativeType;

pub struct CombinedConnector {
    capabilities: Vec<ConnectorCapability>,
}

impl CombinedConnector {
    // returns a connector representing the intersection of all provided connectors
    pub fn new(connectors: Vec<Box<dyn Connector>>) -> Box<dyn Connector> {
        // the standard library does not seem to offer an elegant way to do this. Don't want to pull in a dependency for this.
        let mut combined_capabilities = vec![];
        for connector in &connectors {
            for capability in connector.capabilities() {
                let supported_by_all_connectors = connectors.iter().all(|c| c.has_capability(*capability));

                if supported_by_all_connectors {
                    combined_capabilities.push(*capability);
                }
            }
        }

        Box::new(CombinedConnector {
            capabilities: combined_capabilities,
        })
    }
}

impl Connector for CombinedConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        unimplemented!("A combined connector must not be used for native types")
    }

    fn parse_native_type(&self, _name: &str, _args: Vec<u32>) -> Option<NativeTypeInstance> {
        unimplemented!("A combined connector must not be used for native types")
    }

    fn introspect_native_type(&self, _native_type: Box<dyn NativeType>) -> Option<NativeTypeInstance> {
        unimplemented!("A combined connector must not be used for native types")
    }
}
