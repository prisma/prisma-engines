use crate::error::DatamodelError;
use crate::StringFromEnvVar;

// TODO: Probably rename everything. Terminology here is messy.

/// Trait for custom sources.
///
/// A source is basically the datamodel equivalent of a connector.
pub trait Source {
    /// Gets the name of the implementing connector.
    fn connector_type(&self) -> &str;

    /// Gets the name of the source configuration block.
    fn name(&self) -> &String;

    /// Gets the source config URL.
    fn url(&self) -> &StringFromEnvVar;

    fn set_url(&mut self, url: &str);

    /// Documentation of this source.
    fn documentation(&self) -> &Option<String>;
}

/// Trait for source definitions.
///
/// It provides access to the source's name, as well as a factory method.
pub trait SourceDefinition {
    /// Returns the name of the source.
    fn connector_type(&self) -> &'static str;
    /// Instantiates a new source, using the given name, url and detailed arguments.
    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
    ) -> Result<Box<dyn Source>, DatamodelError>;
}
