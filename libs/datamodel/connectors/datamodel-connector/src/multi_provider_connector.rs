use super::{Connector, ScalarFieldType};
use crate::ConnectorCapability;

pub struct MultiProviderConnector {
    connectors: Vec<Box<dyn Connector>>,
    combined_capabilities: Vec<ConnectorCapability>,
}

impl MultiProviderConnector {
    pub fn new(connectors: Vec<Box<dyn Connector>>) -> MultiProviderConnector {
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

        MultiProviderConnector {
            connectors,
            combined_capabilities,
        }
    }
}

impl Connector for MultiProviderConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.combined_capabilities
    }

    fn calculate_type(&self, _name: &str, _args: Vec<i32>) -> Option<ScalarFieldType> {
        unimplemented!("A combined connector cannot calculate a type yet.")
    }
}
