#![deny(rust_2018_idioms, unsafe_code)]

mod cockroach_datamodel_connector;
mod mssql_datamodel_connector;
mod mysql_datamodel_connector;
mod postgres_datamodel_connector;
mod sqlite_datamodel_connector;

use datamodel_connector::Connector;

pub struct SqlDatamodelConnectors;

impl SqlDatamodelConnectors {
    pub const POSTGRES: &'static dyn Connector = &postgres_datamodel_connector::PostgresDatamodelConnector;
    pub const COCKROACH: &'static dyn Connector = &cockroach_datamodel_connector::CockroachDatamodelConnector;
    pub const MYSQL: &'static dyn Connector = &mysql_datamodel_connector::MySqlDatamodelConnector;
    pub const SQLITE: &'static dyn Connector = &sqlite_datamodel_connector::SqliteDatamodelConnector;
    pub const MSSQL: &'static dyn Connector = &mssql_datamodel_connector::MsSqlDatamodelConnector;
}
