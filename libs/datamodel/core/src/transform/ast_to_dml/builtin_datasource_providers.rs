use super::datasource_provider::DatasourceProvider;
use crate::common::provider_names::*;
use crate::StringFromEnvVar;
use datamodel_connector::Connector;
use mongodb_datamodel_connector::MongoDbDatamodelConnector;
use sql_datamodel_connector::SqlDatamodelConnectors;

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

    fn canonical_name(&self) -> &str {
        SQLITE_SOURCE_NAME
    }

    fn can_handle_url(&self, name: &str, url: &StringFromEnvVar) -> Result<(), String> {
        let high_prio_validation = validate_url(name, "file:", url);
        let low_prio_validation = validate_url(name, "sqlite:", url);
        low_prio_validation.or(high_prio_validation)
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::sqlite())
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

    fn canonical_name(&self) -> &str {
        POSTGRES_SOURCE_NAME
    }

    fn can_handle_url(&self, name: &str, url: &StringFromEnvVar) -> Result<(), String> {
        let high_prio_validation = validate_url(name, "postgresql://", url);
        let low_prio_validation = validate_url(name, "postgres://", url); // for postgres urls on heroku -> https://devcenter.heroku.com/articles/heroku-postgresql#spring-java
        low_prio_validation.or(high_prio_validation)
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::postgres())
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

    fn canonical_name(&self) -> &str {
        MYSQL_SOURCE_NAME
    }

    fn can_handle_url(&self, name: &str, url: &StringFromEnvVar) -> Result<(), String> {
        validate_url(name, "mysql://", url)
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::mysql())
    }
}

pub struct MsSqlDatasourceProvider {}
impl MsSqlDatasourceProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl DatasourceProvider for MsSqlDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MSSQL_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MSSQL_SOURCE_NAME
    }

    fn can_handle_url(&self, name: &str, url: &StringFromEnvVar) -> Result<(), String> {
        validate_url(name, "sqlserver://", url)
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::mssql())
    }
}

pub struct MongoDbDatasourceProvider {}
impl MongoDbDatasourceProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl DatasourceProvider for MongoDbDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MONGODB_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MONGODB_SOURCE_NAME
    }

    fn can_handle_url(&self, name: &str, url: &StringFromEnvVar) -> Result<(), String> {
        validate_url(name, "mongodb://", url)
            .or(validate_url(name, "mongodb+srv://", url))
            .map_err(|_| {
                format!(
                    "The URL for datasource `{}` must start with either the protocol `mongodb://` or `mongodb+srv://`.",
                    name
                )
            })
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(MongoDbDatamodelConnector::new())
    }
}

fn validate_url(name: &str, expected_protocol: &str, url: &StringFromEnvVar) -> Result<(), String> {
    if url.value.starts_with(expected_protocol) {
        Ok(())
    } else {
        Err(format!(
            "The URL for datasource `{}` must start with the protocol `{}`.",
            name, expected_protocol
        ))
    }
}
