use crate::StringFromEnvVar;
use datamodel_connector::Connector;

// TODO: Probably rename everything. Terminology here is messy.

/// Trait for custom sources.
///
/// A source is basically the datamodel equivalent of a connector.
pub trait Source: Send + Sync {
    /// Gets the name of the implementing connector.
    fn connector_type(&self) -> &str;

    /// Gets the name of the source configuration block.
    fn name(&self) -> &String;

    /// Gets the source config URL.
    fn url(&self) -> &StringFromEnvVar;

    fn set_url(&mut self, url: &str);

    /// Documentation of this source.
    fn documentation(&self) -> &Option<String>;

    fn connector(&self) -> &Box<dyn Connector>;
}

pub trait SourceDefinition {
    /// Passes the provider arg from the datasource. Must return true for all provider names it can handle.
    fn is_provider(&self, provider: &str) -> bool;

    /// Instantiates a new source, using the given name, url and detailed arguments.
    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
        connector: Box<dyn Connector>,
    ) -> Result<Box<dyn Source>, String>;
}
