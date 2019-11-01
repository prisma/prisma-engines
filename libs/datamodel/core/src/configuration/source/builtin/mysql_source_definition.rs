use super::{MySqlSource, MYSQL_SOURCE_NAME};
use crate::{common::argument::Arguments, configuration::*, error::DatamodelError};

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
    ) -> Result<Box<dyn Source>, DatamodelError> {
        Ok(Box::new(MySqlSource {
            name: String::from(name),
            url: url,
            documentation: documentation.clone(),
        }))
    }
}
