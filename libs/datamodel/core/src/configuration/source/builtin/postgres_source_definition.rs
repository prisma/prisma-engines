use super::{PostgresSource, POSTGRES_SOURCE_NAME};
use crate::{common::argument::Arguments, configuration::*, error::DatamodelError};

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
    ) -> Result<Box<dyn Source>, DatamodelError> {
        Ok(Box::new(PostgresSource {
            name: String::from(name),
            url: url,
            documentation: documentation.clone(),
        }))
    }
}
