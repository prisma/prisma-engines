use super::{Datasource, DatasourceProvider};
use crate::StringFromEnvVar;
use datamodel_connector::Connector;

pub const SQLITE_SOURCE_NAME: &str = "sqlite";
pub const POSTGRES_SOURCE_NAME: &str = "postgresql";
const POSTGRES_SOURCE_NAME_HEROKU: &str = "postgres";
pub const MYSQL_SOURCE_NAME: &str = "mysql";

pub struct SqliteDatasourceProvider {}

impl SqliteDatasourceProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl DatasourceProvider for SqliteDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == SQLITE_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
        connector: Box<dyn Connector>,
    ) -> Result<Datasource, String> {
        let validation_with_file_protocol = validate_url(name, "file:", url.clone());
        let validation_with_sqlite_protocol = validate_url(name, "sqlite://", url);
        Ok(Datasource {
            name: String::from(name),
            connector_type: SQLITE_SOURCE_NAME.to_owned(),
            url: validation_with_file_protocol.or(validation_with_sqlite_protocol)?,
            documentation: documentation.clone(),
            connector,
        })
    }
}

pub struct PostgresDatasourceProvider {}

impl PostgresDatasourceProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl DatasourceProvider for PostgresDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == POSTGRES_SOURCE_NAME || provider == POSTGRES_SOURCE_NAME_HEROKU
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
        connector: Box<dyn Connector>,
    ) -> Result<Datasource, String> {
        let high_prio_validation = validate_url(name, "postgresql://", url.clone());
        let low_prio_validation = validate_url(name, "postgres://", url); // for postgres urls on heroku -> https://devcenter.heroku.com/articles/heroku-postgresql#spring-java

        Ok(Datasource {
            name: String::from(name),
            connector_type: POSTGRES_SOURCE_NAME.to_owned(),
            url: low_prio_validation.or(high_prio_validation)?,
            documentation: documentation.clone(),
            connector,
        })
    }
}

pub struct MySqlDatasourceProvider {}

impl MySqlDatasourceProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl DatasourceProvider for MySqlDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MYSQL_SOURCE_NAME
    }

    fn create(
        &self,
        name: &str,
        url: StringFromEnvVar,
        documentation: &Option<String>,
        connector: Box<dyn Connector>,
    ) -> Result<Datasource, String> {
        Ok(Datasource {
            name: String::from(name),
            connector_type: MYSQL_SOURCE_NAME.to_owned(),
            url: validate_url(name, "mysql://", url)?,
            documentation: documentation.clone(),
            connector,
        })
    }
}

fn validate_url(name: &str, expected_protocol: &str, url: StringFromEnvVar) -> Result<StringFromEnvVar, String> {
    if url.value.starts_with(expected_protocol) {
        Ok(url)
    } else {
        Err(format!(
            "The URL for datasource `{}` must start with the protocol `{}`.",
            name, expected_protocol
        ))
    }
}
