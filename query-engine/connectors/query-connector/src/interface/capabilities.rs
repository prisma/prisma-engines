/// Contains all capabilities that the connector is able to serve.
#[derive(Debug, Default)]
pub struct ConnectorCapabilities {
    capabilities: Vec<ConnectorCapability>,
}

impl ConnectorCapabilities {
    pub fn add(mut self, capability: ConnectorCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn contains(&self, capability: ConnectorCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

/// Enum describing all possible connector capabilities.
#[derive(Debug, PartialEq)]
pub enum ConnectorCapability {
    InsensitiveFilters,
}
