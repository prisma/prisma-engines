mod sql_server;

use datamodel_connector::ConnectorCapability;
use sql_server::*;
use std::convert::TryFrom;

pub struct ParseError {
    pub reason: String,
}

impl ParseError {
    pub fn new<T>(reason: T) -> Self
    where
        T: Into<String>,
    {
        Self { reason: reason.into() }
    }
}

pub trait ConnectorTagInterface {
    /// The connection string to use to connect to the test database and version.
    fn connection_string(&self) -> String;

    /// Capabilities of the implementing connector.
    fn capabilities(&self) -> Vec<ConnectorCapability>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectorTag {
    SqlServer(SqlServerConnectorTag),
    MySql,
    Postgres,
    Sqlite,
    MongoDb,
}

impl ConnectorTag {
    pub fn all() -> Vec<Self> {
        SqlServerConnectorTag::all().into_iter().map(Self::SqlServer).collect()
    }
}

impl TryFrom<&str> for ConnectorTag {
    type Error = ParseError;

    fn try_from(tag: &str) -> Result<Self, Self::Error> {
        Self::try_from((tag, None))
    }
}

impl TryFrom<(&str, Option<&str>)> for ConnectorTag {
    type Error = ParseError;

    fn try_from(value: (&str, Option<&str>)) -> Result<Self, Self::Error> {
        let (connector, version) = value;

        let tag = match connector.to_lowercase().as_str() {
            "sqlserver" => Self::SqlServer(SqlServerConnectorTag::new(version)?),
            "mysql" => Self::MySql,
            "postgres" => Self::Postgres,
            "sqlite" => Self::Sqlite,
            "mongodb" => Self::MongoDb,
            _ => return Err(ParseError::new(format!("Unknown connector tag `{}`", connector))),
        };

        Ok(tag)
    }
}

#[derive(Debug)]
pub enum MySqlVersion {
    V5_6,
    V5_7,
    V8,
}

#[derive(Debug)]
pub enum PostgresVersion {
    V9,
    V10,
    V11,
    V12,
}
