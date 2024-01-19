pub mod cockroach_datamodel_connector;
pub mod completions;

pub use cockroach_datamodel_connector::CockroachType;
pub use mongodb::MongoDbType;
pub use mssql_datamodel_connector::{MsSqlType, MsSqlTypeParameter};
pub use mysql_datamodel_connector::MySqlType;
pub use postgres_datamodel_connector::{PostgresDatasourceProperties, PostgresType};

mod mongodb;
mod mssql_datamodel_connector;
mod mysql_datamodel_connector;
mod native_type_definition;
mod postgres_datamodel_connector;
mod sqlite_datamodel_connector;
mod utils;

use crate::{datamodel_connector::Connector, ConnectorRegistry};

pub const POSTGRES: &'static dyn Connector = &postgres_datamodel_connector::PostgresDatamodelConnector;
pub const COCKROACH: &'static dyn Connector = &cockroach_datamodel_connector::CockroachDatamodelConnector;
pub const MYSQL: &'static dyn Connector = &mysql_datamodel_connector::MySqlDatamodelConnector;
pub const SQLITE: &'static dyn Connector = &sqlite_datamodel_connector::SqliteDatamodelConnector;
pub const MSSQL: &'static dyn Connector = &mssql_datamodel_connector::MsSqlDatamodelConnector;
pub const MONGODB: &'static dyn Connector = &mongodb::MongoDbDatamodelConnector;

pub static BUILTIN_CONNECTORS: ConnectorRegistry<'static> = &[POSTGRES, MYSQL, SQLITE, MSSQL, COCKROACH, MONGODB];
