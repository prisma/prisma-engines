use super::shared_validation::*;
use crate::configuration::source::builtin::SimpleSource;
use crate::configuration::*;
use datamodel_connector::Connector;

pub const SQLITE_SOURCE_NAME: &str = "sqlite";

pub struct SqliteSourceDefinition {}

impl SqliteSourceDefinition {
    pub fn new() -> Self {
        Self {}
    }
}

impl SourceDefinition for SqliteSourceDefinition {
    fn is_provider(&self, provider: &str) -> bool {
        provider == SQLITE_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
        connector: Box<dyn Connector>,
    ) -> Result<Box<dyn Source + Send + Sync>, String> {
        let validation_with_file_protocol = validate_url(name, "file:", url.clone());
        let validation_with_sqlite_protocol = validate_url(name, "sqlite://", url);
        Ok(Box::new(SimpleSource {
            name: String::from(name),
            connector_type: SQLITE_SOURCE_NAME.to_owned(),
            url: validation_with_file_protocol.or(validation_with_sqlite_protocol)?,
            documentation: documentation.clone(),
            connector,
        }))
    }
}
