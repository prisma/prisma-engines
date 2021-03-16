use std::str::FromStr;

use super::*;

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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlServerVersion {
    V2017,
    V2019,
}

impl SqlServerConnectorTag {
    pub fn new(version: Option<&str>) -> Result<Self, ParseError> {
        let version = match version {
            Some(v) => Some(v.parse()?),
            None => None,
        };

        Ok(Self { version })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                version: Some(SqlServerVersion::V2017),
            },
            Self {
                version: Some(SqlServerVersion::V2019),
            },
        ]
    }
}

impl FromStr for SqlServerVersion {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version = match s {
            "2017" => Self::V2017,
            "2019" => Self::V2019,
            _ => return Err(ParseError::new(format!("Unknown SqlServer version `{}`", s))),
        };

        Ok(version)
    }
}
