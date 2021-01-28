use super::Connector;
use crate::{connector_error::ConnectorError, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance};
use dml::{field::Field, model::Model, scalars::ScalarType};
use std::unimplemented;

pub struct CombinedConnector {
    capabilities: Vec<ConnectorCapability>,
}

impl CombinedConnector {
    // returns a connector representing the intersection of all provided connectors
    pub fn new(connectors: Vec<Box<dyn Connector>>) -> Self {
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

        CombinedConnector {
            capabilities: combined_capabilities,
        }
    }
}

impl Connector for CombinedConnector {
    fn name(&self) -> String {
        unimplemented!("A combined connector does not have a name")
    }

    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn validate_field(&self, _field: &Field) -> Result<(), ConnectorError> {
        Ok(())
    }

    fn validate_model(&self, _model: &Model) -> Result<(), ConnectorError> {
        Ok(())
    }

    fn native_type_is_default_for_scalar_type(
        &self,
        _native_type: serde_json::Value,
        _scalar_type: &ScalarType,
    ) -> bool {
        unimplemented!("A combined connector must not be used for native types")
    }

    fn available_native_type_constructors(&self) -> &[NativeTypeConstructor] {
        unimplemented!("A combined connector must not be used for native types")
    }

    fn parse_native_type(&self, _name: &str, _args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError> {
        unimplemented!("A combined connector must not be used for native types")
    }

    fn introspect_native_type(&self, _native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError> {
        unimplemented!("A combined connector must not be used for native types")
    }

    fn default_native_type_for_scalar_type(&self, _scalar_type: &ScalarType) -> serde_json::Value {
        unimplemented!("A combined connector must not be used for native types")
    }
}
