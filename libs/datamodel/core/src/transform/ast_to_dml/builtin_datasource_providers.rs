use super::datasource_provider::DatasourceProvider;
use crate::common::provider_names::*;
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

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::postgres())
    }
}

pub struct MySqlDatasourceProvider {
    is_planetscale: bool,
}

impl MySqlDatasourceProvider {
    pub fn new(is_planetscale: bool) -> Self {
        Self { is_planetscale }
    }
}

impl DatasourceProvider for MySqlDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MYSQL_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MYSQL_SOURCE_NAME
    }

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(SqlDatamodelConnectors::mysql(self.is_planetscale))
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

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(MongoDbDatamodelConnector::new())
    }
}
