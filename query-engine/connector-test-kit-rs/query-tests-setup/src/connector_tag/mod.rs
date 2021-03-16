mod sql_server;

use datamodel_connector::ConnectorCapability;
use sql_server::*;
use std::convert::TryFrom;

use crate::TestError;

pub trait ConnectorTagInterface {
    /// The connection string to use to connect to the test database and version.
    fn connection_string(&self) -> String;

    /// Capabilities of the implementing connector.
    fn capabilities(&self) -> Vec<ConnectorCapability>;

    fn as_parse_pair(&self) -> (String, Option<String>);
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
            "mysql" => Self::MySql,
            "postgres" => Self::Postgres,
            "sqlite" => Self::Sqlite,
            "mongodb" => Self::MongoDb,
            _ => return Err(TestError::parse_error(format!("Unknown connector tag `{}`", connector))),
        };

        Ok(tag)
    }
}

impl ConnectorTagInterface for ConnectorTag {
    fn connection_string(&self) -> String {
        match self {
            ConnectorTag::SqlServer(tag) => tag.connection_string(),
            ConnectorTag::MySql => todo!(),
            ConnectorTag::Postgres => todo!(),
            ConnectorTag::Sqlite => todo!(),
            ConnectorTag::MongoDb => todo!(),
        }
    }

    fn capabilities(&self) -> Vec<ConnectorCapability> {
        match self {
            ConnectorTag::SqlServer(tag) => tag.capabilities(),
            ConnectorTag::MySql => todo!(),
            ConnectorTag::Postgres => todo!(),
            ConnectorTag::Sqlite => todo!(),
            ConnectorTag::MongoDb => todo!(),
        }
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        match self {
            ConnectorTag::SqlServer(tag) => tag.as_parse_pair(),
            ConnectorTag::MySql => todo!(),
            ConnectorTag::Postgres => todo!(),
            ConnectorTag::Sqlite => todo!(),
            ConnectorTag::MongoDb => todo!(),
        }
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
