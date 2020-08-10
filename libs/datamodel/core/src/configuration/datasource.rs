use datamodel_connector::Connector;
use serde::Serialize;

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
    pub preview_features: Vec<String>,
}

impl Datasource {
    pub fn url(&self) -> &StringFromEnvVar {
        &self.url
    }
}

pub trait PreviewFeatures {
    fn has_preview_feature(&self, feature: &str) -> bool;
}

impl PreviewFeatures for Datasource {
    fn has_preview_feature(&self, feature: &str) -> bool {
        self.preview_features.contains(&feature.to_string())
    }
}

impl PreviewFeatures for Option<&Datasource> {
    fn has_preview_feature(&self, feature: &str) -> bool {
        match self {
            Some(dat) => dat.has_preview_feature(feature),
            None => false,
        }
    }
}

#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct StringFromEnvVar {
    /// contains the name of env var if the value was read from one
    pub from_env_var: Option<String>,
    pub value: String,
}
