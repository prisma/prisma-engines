use datamodel_connector::{Connector, ConnectorCapabilities};
use serde::Serialize;

/// a `datasource` from the prisma schema.
pub struct Datasource {
    pub name: String,
    /// all providers that were specified
    pub provider: Vec<String>,
    /// the provider that was selected as active from all specified providers
    pub active_provider: String,
    pub url: StringFromEnvVar,
    // pub shadow_database_url: StringFromEnvVar,
    pub documentation: Option<String>,
    /// a connector representing the intersection of all providers specified
    pub combined_connector: Box<dyn Connector>,
    /// the connector of the active provider
    pub active_connector: Box<dyn Connector>,
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

#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct StringFromEnvVar {
    /// contains the name of env var if the value was read from one
    pub from_env_var: Option<String>,
    pub value: String,
}
