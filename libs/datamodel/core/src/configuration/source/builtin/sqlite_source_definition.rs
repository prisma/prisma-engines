use super::{shared_validation::*, SqliteSource, SQLITE_SOURCE_NAME};
use crate::configuration::*;

pub struct SqliteSourceDefinition {}

impl SqliteSourceDefinition {
    pub fn new() -> Self {
        Self {}
    }
}

impl SourceDefinition for SqliteSourceDefinition {
    fn connector_type(&self) -> &'static str {
        SQLITE_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
    ) -> Result<Box<dyn Source + Send + Sync>, String> {
        let validation_with_file_protocol = validate_url(name, "file://", url.clone());
        let validation_with_sqlite_protocol = validate_url(name, "sqlite://", url);
        Ok(Box::new(SqliteSource {
            name: String::from(name),
            url: validation_with_file_protocol.or(validation_with_sqlite_protocol)?,
            documentation: documentation.clone(),
        }))
    }
}
