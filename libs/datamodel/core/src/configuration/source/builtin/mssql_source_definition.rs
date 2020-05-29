use super::{shared_validation::*, MSSqlSource, MSSQL_SOURCE_NAME};
use crate::configuration::*;

pub struct MSSqlSourceDefinition {}

impl MSSqlSourceDefinition {
    pub fn new() -> Self {
        Self {}
    }
}

impl SourceDefinition for MSSqlSourceDefinition {
    fn connector_type(&self) -> &'static str {
        MSSQL_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
    ) -> Result<Box<dyn Source + Send + Sync>, String> {
        Ok(Box::new(MSSqlSource {
            name: String::from(name),
            url: validate_url(name, "sqlserver://", url)?,
            documentation: documentation.clone(),
        }))
    }
}
