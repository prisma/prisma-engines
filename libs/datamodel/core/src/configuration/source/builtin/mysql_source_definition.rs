use super::shared_validation::*;
use crate::configuration::source::builtin::SimpleSource;
use crate::configuration::*;
use datamodel_connector::Connector;

pub const MYSQL_SOURCE_NAME: &str = "mysql";

pub struct MySqlSourceDefinition {}

impl MySqlSourceDefinition {
    pub fn new() -> Self {
        Self {}
    }
}

impl SourceDefinition for MySqlSourceDefinition {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MYSQL_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
        connector: Box<dyn Connector>,
    ) -> Result<Box<dyn Source>, String> {
        Ok(Box::new(SimpleSource {
            name: String::from(name),
            connector_type: MYSQL_SOURCE_NAME.to_owned(),
            url: validate_url(name, "mysql://", url)?,
            documentation: documentation.clone(),
            connector,
        }))
    }
}
