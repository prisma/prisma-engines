use super::datasource_provider::DatasourceProvider;
use crate::common::provider_names::*;
use datamodel_connector::Connector;
use mongodb_datamodel_connector::MongoDbDatamodelConnector;
use sql_datamodel_connector::SqlDatamodelConnectors;

pub struct SqliteDatasourceProvider;

impl DatasourceProvider for SqliteDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == SQLITE_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        SQLITE_SOURCE_NAME
    }

    fn connector(&self) -> &'static dyn Connector {
        SqlDatamodelConnectors::SQLITE
    }
}

pub struct PostgresDatasourceProvider;

impl DatasourceProvider for PostgresDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == POSTGRES_SOURCE_NAME || provider == POSTGRES_SOURCE_NAME_HEROKU
    }

    fn canonical_name(&self) -> &str {
        POSTGRES_SOURCE_NAME
    }

    fn connector(&self) -> &'static dyn Connector {
        SqlDatamodelConnectors::POSTGRES
    }
}

pub struct MySqlDatasourceProvider;

impl DatasourceProvider for MySqlDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MYSQL_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MYSQL_SOURCE_NAME
    }

    fn connector(&self) -> &'static dyn Connector {
        SqlDatamodelConnectors::MYSQL
    }
}

pub struct MsSqlDatasourceProvider;

impl DatasourceProvider for MsSqlDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MSSQL_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MSSQL_SOURCE_NAME
    }

    fn connector(&self) -> &'static dyn Connector {
        SqlDatamodelConnectors::MSSQL
    }
}

pub struct MongoDbDatasourceProvider;

impl DatasourceProvider for MongoDbDatasourceProvider {
    fn is_provider(&self, provider: &str) -> bool {
        provider == MONGODB_SOURCE_NAME
    }

    fn canonical_name(&self) -> &str {
        MONGODB_SOURCE_NAME
    }

    fn connector(&self) -> &'static dyn Connector {
        &MongoDbDatamodelConnector
    }
}
