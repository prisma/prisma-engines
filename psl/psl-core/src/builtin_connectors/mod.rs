#[cfg(feature = "cockroachdb")]
pub mod cockroach_datamodel_connector;
pub mod completions;
pub mod geometry;

#[cfg(feature = "cockroachdb")]
pub use cockroach_datamodel_connector::CockroachType;
#[cfg(feature = "mongodb")]
pub use mongodb::MongoDbType;
#[cfg(feature = "mssql")]
pub use mssql_datamodel_connector::{MsSqlType, MsSqlTypeParameter};
#[cfg(feature = "mysql")]
pub use mysql_datamodel_connector::MySqlType;
#[cfg(feature = "postgresql")]
pub use postgres_datamodel_connector::{PostgresDatasourceProperties, PostgresType};
#[cfg(feature = "sqlite")]
pub use sqlite_datamodel_connector::SQLiteType;

mod capabilities_support;
#[cfg(feature = "mongodb")]
mod mongodb;
#[cfg(feature = "mssql")]
mod mssql_datamodel_connector;
#[cfg(feature = "mysql")]
mod mysql_datamodel_connector;
mod native_type_definition;
#[cfg(feature = "postgresql")]
mod postgres_datamodel_connector;
#[cfg(feature = "sqlite")]
mod sqlite_datamodel_connector;
mod utils;
pub use capabilities_support::{can_have_capability, can_support_relation_load_strategy, has_capability};

use crate::ConnectorRegistry;

#[cfg(feature = "postgresql")]
pub const POSTGRES: &'static dyn crate::datamodel_connector::Connector =
    &postgres_datamodel_connector::PostgresDatamodelConnector;
#[cfg(feature = "cockroachdb")]
pub const COCKROACH: &'static dyn crate::datamodel_connector::Connector =
    &cockroach_datamodel_connector::CockroachDatamodelConnector;
#[cfg(feature = "mysql")]
pub const MYSQL: &'static dyn crate::datamodel_connector::Connector =
    &mysql_datamodel_connector::MySqlDatamodelConnector;
#[cfg(feature = "sqlite")]
pub const SQLITE: &'static dyn crate::datamodel_connector::Connector =
    &sqlite_datamodel_connector::SqliteDatamodelConnector;
#[cfg(feature = "mssql")]
pub const MSSQL: &'static dyn crate::datamodel_connector::Connector =
    &mssql_datamodel_connector::MsSqlDatamodelConnector;
#[cfg(feature = "mongodb")]
pub const MONGODB: &'static dyn crate::datamodel_connector::Connector = &mongodb::MongoDbDatamodelConnector;

pub static BUILTIN_CONNECTORS: ConnectorRegistry<'static> = &[
    #[cfg(feature = "postgresql")]
    POSTGRES,
    #[cfg(feature = "mysql")]
    MYSQL,
    #[cfg(feature = "sqlite")]
    SQLITE,
    #[cfg(feature = "mssql")]
    MSSQL,
    #[cfg(feature = "cockroachdb")]
    COCKROACH,
    #[cfg(feature = "mongodb")]
    MONGODB,
];
