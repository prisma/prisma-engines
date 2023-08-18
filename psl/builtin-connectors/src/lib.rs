#![deny(rust_2018_idioms, unsafe_code)]
#![allow(clippy::derive_partial_eq_without_eq)]

pub mod cockroach_datamodel_connector;
pub mod completions;

pub use cockroach_datamodel_connector::CockroachType;
pub use mongodb::MongoDbType;
pub use mssql_datamodel_connector::{MsSqlType, MsSqlTypeParameter};
pub use mysql_datamodel_connector::MySqlType;
pub use postgres_datamodel_connector::{PostgresDatasourceProperties, PostgresType};
pub use psl_core::js_connector::JsConnector;
pub use pg_js::PG_JS;

mod mongodb;
mod mssql_datamodel_connector;
mod mysql_datamodel_connector;
mod native_type_definition;
mod neon;
mod pg_js;
mod planetscale;
mod postgres_datamodel_connector;
mod sqlite_datamodel_connector;

use psl_core::{datamodel_connector::Connector, ConnectorRegistry};

pub const POSTGRES: &'static dyn Connector = &postgres_datamodel_connector::PostgresDatamodelConnector;
pub const COCKROACH: &'static dyn Connector = &cockroach_datamodel_connector::CockroachDatamodelConnector;
pub const MYSQL: &'static dyn Connector = &mysql_datamodel_connector::MySqlDatamodelConnector;
pub const SQLITE: &'static dyn Connector = &sqlite_datamodel_connector::SqliteDatamodelConnector;
pub const MSSQL: &'static dyn Connector = &mssql_datamodel_connector::MsSqlDatamodelConnector;
pub const MONGODB: &'static dyn Connector = &mongodb::MongoDbDatamodelConnector;
pub static PLANETSCALE_SERVERLESS: &'static dyn Connector = &planetscale::PLANETSCALE_SERVERLESS;
pub static NEON_SERVERLESS: &'static dyn Connector = &neon::NEON_SERVERLESS;

pub static BUILTIN_CONNECTORS: ConnectorRegistry = &[
    POSTGRES,
    MYSQL,
    SQLITE,
    MSSQL,
    COCKROACH,
    &PG_JS,
    MONGODB,
    PLANETSCALE_SERVERLESS,
    NEON_SERVERLESS,
];
