use super::{shared_validation::*, MySqlSource, MYSQL_SOURCE_NAME};
use crate::configuration::*;

pub struct MySqlSourceDefinition {}

impl MySqlSourceDefinition {
    pub fn new() -> Self {
        Self {}
    }
}

impl SourceDefinition for MySqlSourceDefinition {
    fn connector_type(&self) -> &'static str {
        MYSQL_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
    ) -> Result<Box<dyn Source + Send + Sync>, String> {
        Ok(Box::new(MySqlSource {
            name: String::from(name),
            url: validate_url(name, "mysql://", url)?,
            documentation: documentation.clone(),
        }))
    }
}
