use super::DatasourceProvider;
use crate::StringFromEnvVar;
use datamodel_connector::{BuiltinConnectors, Connector};

pub const SQLITE_SOURCE_NAME: &str = "sqlite";
pub const POSTGRES_SOURCE_NAME: &str = "postgresql";
const POSTGRES_SOURCE_NAME_HEROKU: &str = "postgres";
pub const MYSQL_SOURCE_NAME: &str = "mysql";
pub const MSSQL_SOURCE_NAME: &str = "sqlserver";

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
        let validation_with_file_protocol = validate_url(name, "file:", url);
        let validation_with_sqlite_protocol = validate_url(name, "sqlite://", url);
        validation_with_file_protocol.or(validation_with_sqlite_protocol)
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(BuiltinConnectors::sqlite())
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
        Box::new(BuiltinConnectors::postgres())
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
        Box::new(BuiltinConnectors::mysql())
    }
}

pub struct MsSqlDatasourceProvider {}
impl MsSqlDatasourceProvider {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(feature = "mssql")]
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
        Box::new(BuiltinConnectors::mssql())
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
