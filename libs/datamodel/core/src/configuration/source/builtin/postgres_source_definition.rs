use super::shared_validation::*;
use crate::configuration::source::builtin::SimpleSource;
use crate::configuration::*;
use datamodel_connector::Connector;

pub const POSTGRES_SOURCE_NAME: &str = "postgresql";

pub struct PostgresSourceDefinition {}

impl PostgresSourceDefinition {
    pub fn new() -> Self {
        Self {}
    }
}

impl SourceDefinition for PostgresSourceDefinition {
    fn is_provider(&self, provider: &str) -> bool {
        provider == POSTGRES_SOURCE_NAME || provider == "postgres"
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
        connector: Box<dyn Connector>,
    ) -> Result<Box<dyn Source>, String> {
        let high_prio_validation = validate_url(name, "postgresql://", url.clone());
        let low_prio_validation = validate_url(name, "postgres://", url); // for postgres urls on heroku -> https://devcenter.heroku.com/articles/heroku-postgresql#spring-java

        Ok(Box::new(SimpleSource {
            name: String::from(name),
            connector_type: POSTGRES_SOURCE_NAME.to_owned(),
            url: low_prio_validation.or(high_prio_validation)?,
            documentation: documentation.clone(),
            connector,
        }))
    }
}
