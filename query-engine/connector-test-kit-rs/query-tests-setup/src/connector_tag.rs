use std::str::FromStr;

use datamodel_connector::ConnectorCapability;

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

    ///
    fn parse_from(connector_str: &str, version_str: Option<&str>) -> ConnectorTag {
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectorTag {
    SqlServer(SqlServerConnectorTag),
    MySql,
    Postgres,
    Sqlite,
    MongoDb,
}

// impl ConnectorTagInterface for ConnectorTag {
//     fn connection_string(&self) -> String {
//         todo!()
//     }

//     fn capabilities(&self) -> Vec<ConnectorCapability> {
//         todo!()
//     }

//     fn parse_from(connector_str: &str, version_str: Option<&str>) -> ConnectorTag {

//     }
// }

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SqlServerConnectorTag {
    version: Option<SqlServerVersion>,
}

impl ConnectorTagInterface for SqlServerConnectorTag {
    fn connection_string(&self) -> String {
        todo!()
    }

    fn capabilities(&self) -> Vec<ConnectorCapability> {
        todo!()
    }

    fn parse_from(connector_str: &str, version_str: Option<&str>) -> ConnectorTag {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlServerVersion {
    V_2017,
    V_2019,
}

impl FromStr for SqlServerVersion {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2017" => todo!(),
            "2019" => todo!(),
            _ => return Err(ParseError::new(format!("Unknown SqlServer version `{}`", s))),
        }
    }
}

impl FromStr for ConnectorTag {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tag = match s {
            "sqlserver" => Self::SqlServer(SqlServerConnectorTag::default()),
            "mysql" => Self::MySql,
            "postgres" => Self::Postgres,
            "sqlite" => Self::Sqlite,
            "mongodb" => Self::MongoDb,
            _ => return Err(ParseError::new(format!("Unknown connector tag `{}`", s))),
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
