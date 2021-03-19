mod postgres;
mod sql_schema_renderer;
mod sql_server;

use datamodel_connector::ConnectorCapability;
use enum_dispatch::enum_dispatch;
use postgres::*;
use sql_server::*;
use std::convert::TryFrom;

use crate::{TestConfig, TestError};

#[enum_dispatch]
pub trait ConnectorTagInterface {
    /// The name of the datamodel provider for this connector.
    /// Must match valid datamodel provider strings.
    fn datamodel_provider(&self) -> &'static str;

    /// Renders the test datamodel (the models portion) based on the passed template.
    fn render_datamodel(&self, template: String) -> String;

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
    // MySql,
    // Sqlite,
    // MongoDb,
}

impl ConnectorTag {
    /// Returns all possible connector tags.
    pub fn all() -> Vec<Self> {
        SqlServerConnectorTag::all()
            .into_iter()
            .map(Self::SqlServer)
            .chain(PostgresConnectorTag::all().into_iter().map(Self::Postgres))
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
            "sqlserver" => Self::SqlServer(SqlServerConnectorTag::new(version)?),
            "postgres" => Self::Postgres(PostgresConnectorTag::new(version)?),
            // "mysql" => Self::MySql,
            // "sqlite" => Self::Sqlite,
            // "mongodb" => Self::MongoDb,
            _ => return Err(TestError::parse_error(format!("Unknown connector tag `{}`", connector))),
        };

        Ok(tag)
    }
}

// #[derive(Debug)]
// pub enum MySqlVersion {
//     V5_6,
//     V5_7,
//     V8,
// }
