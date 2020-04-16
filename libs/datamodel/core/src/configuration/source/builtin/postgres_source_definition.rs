use super::{shared_validation::*, PostgresSource, POSTGRES_SOURCE_NAME};
use crate::configuration::*;

pub struct PostgresSourceDefinition {}

impl PostgresSourceDefinition {
    pub fn new() -> Self {
        Self {}
    }
}

impl SourceDefinition for PostgresSourceDefinition {
    fn connector_type(&self) -> &'static str {
        POSTGRES_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
    ) -> Result<Box<dyn Source + Send + Sync>, String> {
        Ok(Box::new(PostgresSource {
            name: String::from(name),
            url: validate_url(name, "postgresql://", url)?,
            documentation: documentation.clone(),
        }))
    }
}
