use crate::{Datasource, StringFromEnvVar};
use datamodel_connector::Connector;

pub trait DatasourceProvider {
    /// Passes the provider arg from the datasource. Must return true for all provider names it can handle.
    fn is_provider(&self, provider: &str) -> bool;

    /// Instantiates a new source
    fn create(
        &self,
        name: &str,
        provider: Vec<String>,
        url: StringFromEnvVar,
        documentation: &Option<String>,
        connector: Box<dyn Connector>,
    ) -> Result<Datasource, String>;
}
