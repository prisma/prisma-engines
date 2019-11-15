use super::{SqliteSource, SQLITE_SOURCE_NAME};
use crate::{configuration::*, error::DatamodelError};

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
    ) -> Result<Box<dyn Source>, DatamodelError> {
        Ok(Box::new(SqliteSource {
            name: String::from(name),
            url: url,
            documentation: documentation.clone(),
        }))
    }
}
