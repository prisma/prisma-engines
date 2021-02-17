use crate::StringFromEnvVar;
use datamodel_connector::Connector;

pub trait DatasourceProvider {
    /// Passes the provider arg from the datasource. Must return true for all provider names it can handle.
    fn is_provider(&self, provider: &str) -> bool;

    fn canonical_name(&self) -> &str;

    fn validate_url(&self, name: &str, url: &StringFromEnvVar) -> Result<(), String>;

    fn validate_shadow_database_url(&self, name: &str, url: &StringFromEnvVar) -> Result<(), String>;

    fn connector(&self) -> Box<dyn Connector>;
}
