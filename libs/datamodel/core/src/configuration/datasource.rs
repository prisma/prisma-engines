use crate::StringFromEnvVar;
use datamodel_connector::{Connector, ConnectorCapabilities};

/// a `datasource` from the prisma schema.
pub struct Datasource {
    pub name: String,
    /// all providers that were specified
    pub provider: Vec<String>,
    /// the provider that was selected as active from all specified providers
    pub active_provider: String,
    pub url: StringFromEnvVar,
    pub documentation: Option<String>,
    /// a connector representing the intersection of all providers specified
    pub combined_connector: Box<dyn Connector>,
    /// the connector of the active provider
    pub active_connector: Box<dyn Connector>,
    /// An optional user-defined shadow database URL.
    pub shadow_database_url: Option<StringFromEnvVar>,
}

impl std::fmt::Debug for Datasource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Datasource")
            .field("name", &self.name)
            .field("provider", &self.provider)
            .field("active_provider", &self.active_provider)
            .field("url", &self.url)
            .field("documentation", &self.documentation)
            .field("active_connector", &&"...")
            .field("shadow_database_url", &self.shadow_database_url)
            .finish()
    }
}

impl Datasource {
    pub fn url(&self) -> &StringFromEnvVar {
        &self.url
    }

    pub fn capabilities(&self) -> ConnectorCapabilities {
        let capabilities = self.active_connector.capabilities().clone();
        ConnectorCapabilities::new(capabilities)
    }
}
