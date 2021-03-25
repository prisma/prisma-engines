mod mongodb;
mod mysql;
mod postgres;
mod sql_server;
mod sqlite;

use datamodel_connector::ConnectorCapability;
use enum_dispatch::enum_dispatch;
use mongodb::*;
use mysql::*;
use postgres::*;
use sql_server::*;
use sqlite::*;
use std::convert::TryFrom;

use crate::{datamodel_rendering::DatamodelRenderer, TestConfig, TestError};

#[enum_dispatch]
pub trait ConnectorTagInterface {
    /// The name of the datamodel provider for this connector.
    /// Must match valid datamodel provider strings.
    fn datamodel_provider(&self) -> &'static str;

    /// Returns the renderer to be used for templating the datamodel (the models portion).
    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer>;

    /// The connection string to use to connect to the test database and version.
    /// - `test_database` is the database to connect to, which is an implementation detail of the
    ///   implementing connector, like a file or a schema.
    /// - `is_ci` signals whether or not the test run is done on CI or not. May be important if local
    ///   test run connection strings and CI connection strings differ because of networking.
    fn connection_string(&self, test_database: &str, is_ci: bool) -> String;

    /// Capabilities of the implementing connector.
    fn capabilities(&self) -> Vec<ConnectorCapability>;

    /// Serialization of the connector. Expected to return `(tag_name, version)`.
    /// Todo: Think of something better.
    fn as_parse_pair(&self) -> (String, Option<String>);

    /// Must return `true` if the connector family is versioned (e.g. Postgres9, Postgres10, ...), false otherwise.
    fn is_versioned(&self) -> bool;
}

#[enum_dispatch(ConnectorTagInterface)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectorTag {
    SqlServer(SqlServerConnectorTag),
    Postgres(PostgresConnectorTag),
    MySql(MySqlConnectorTag),
    MongoDb(MongoDbConnectorTag),
    Sqlite(SqliteConnectorTag),
}

impl ConnectorTag {
    /// Returns all possible connector tags.
    pub fn all() -> Vec<Self> {
        SqlServerConnectorTag::all()
            .into_iter()
            .map(Self::SqlServer)
            .chain(PostgresConnectorTag::all().into_iter().map(Self::Postgres))
            .chain(MySqlConnectorTag::all().into_iter().map(Self::MySql))
            .chain(MongoDbConnectorTag::all().into_iter().map(Self::MongoDb))
            .chain(SqliteConnectorTag::all().into_iter().map(Self::Sqlite))
            .collect()
    }

    /// Based on the connector tags that are enabled for a test, check if
    /// the current configuration allows for this test to run.
    pub fn should_run(config: &TestConfig, enabled: &[ConnectorTag]) -> bool {
        let current_connector = config.test_connector_tag().unwrap();
        enabled.contains(&current_connector)
    }
}

impl TryFrom<&str> for ConnectorTag {
    type Error = TestError;

    fn try_from(tag: &str) -> Result<Self, Self::Error> {
        Self::try_from((tag, None))
    }
}

impl TryFrom<(&str, Option<&str>)> for ConnectorTag {
    type Error = TestError;

    fn try_from(value: (&str, Option<&str>)) -> Result<Self, Self::Error> {
        let (connector, version) = value;

        let tag = match connector.to_lowercase().as_str() {
            "sqlite" => Self::Sqlite(SqliteConnectorTag::new()),
            "sqlserver" => Self::SqlServer(SqlServerConnectorTag::new(version)?),
            "postgres" => Self::Postgres(PostgresConnectorTag::new(version)?),
            "mysql" => Self::MySql(MySqlConnectorTag::new(version)?),
            "mongodb" => Self::MongoDb(MongoDbConnectorTag::new(version)?),
            _ => return Err(TestError::parse_error(format!("Unknown connector tag `{}`", connector))),
        };

        Ok(tag)
    }
}
