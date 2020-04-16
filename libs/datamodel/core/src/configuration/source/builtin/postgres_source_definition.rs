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
        let high_prio_validation = validate_url(name, "postgresql://", url.clone());
        let low_prio_validation = validate_url(name, "postgres://", url); // for postgres urls on heroku -> https://devcenter.heroku.com/articles/heroku-postgresql#spring-java
        Ok(Box::new(PostgresSource {
            name: String::from(name),
            url: low_prio_validation.or(high_prio_validation)?,
            documentation: documentation.clone(),
        }))
    }
}
