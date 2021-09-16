use datamodel_connector::Connector;
use sql_datamodel_connector::MySqlDatamodelConnector;

use super::*;
use crate::{datamodel_rendering::SqlDatamodelRenderer, TestError, TestResult};

#[derive(Debug, Default, Clone)]
pub struct MySqlConnectorTag {
    version: Option<MySqlVersion>,
    capabilities: Vec<ConnectorCapability>,
}

impl MySqlConnectorTag {
    /// Get a reference to the MySQL connector tag's version.
    pub fn version(&self) -> Option<&MySqlVersion> {
        self.version.as_ref()
    }
}

impl ConnectorTagInterface for MySqlConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "mysql"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(&self, database: &str, is_ci: bool) -> String {
        match self.version {
            Some(MySqlVersion::V5_6) if is_ci => format!("mysql://root:prisma@test-db-mysql-5-6:3306/{}", database),
            Some(MySqlVersion::V5_7) if is_ci => format!("mysql://root:prisma@test-db-mysql-5-7:3306/{}", database),
            Some(MySqlVersion::V8) if is_ci => format!("mysql://root:prisma@test-db-mysql-8-0:3306/{}", database),
            Some(MySqlVersion::MariaDb) if is_ci => format!("mysql://root:prisma@test-db-mariadb:3306/{}", database),
            Some(MySqlVersion::V5_6) => format!("mysql://root:prisma@127.0.0.1:3309/{}", database),
            Some(MySqlVersion::V5_7) => format!("mysql://root:prisma@127.0.0.1:3306/{}", database),
            Some(MySqlVersion::V8) => format!("mysql://root:prisma@127.0.0.1:3307/{}", database),
            Some(MySqlVersion::MariaDb) => {
                format!("mysql://root:prisma@127.0.0.1:3308/{}", database)
            }

            None => unreachable!("A versioned connector must have a concrete version to run."),
        }
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.as_ref().map(ToString::to_string);
        ("mysql".to_owned(), version)
    }

    fn is_versioned(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MySqlVersion {
    V5_6,
    V5_7,
    V8,
    MariaDb,
}

impl MySqlConnectorTag {
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(MySqlVersion::try_from(v)?),
            None => None,
        };

        Ok(Self {
            version,
            capabilities: mysql_capabilities(),
        })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        let capabilities = mysql_capabilities();

        vec![
            Self {
                version: Some(MySqlVersion::V5_6),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(MySqlVersion::V5_7),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(MySqlVersion::V8),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(MySqlVersion::MariaDb),
                capabilities,
            },
        ]
    }
}

impl PartialEq for MySqlConnectorTag {
    fn eq(&self, other: &Self) -> bool {
        match (self.version, other.version) {
            (None, None) | (Some(_), None) | (None, Some(_)) => true,
            (Some(v1), Some(v2)) => v1 == v2,
        }
    }
}

impl TryFrom<&str> for MySqlVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "5.6" => Self::V5_6,
            "5.7" => Self::V5_7,
            "8" => Self::V8,
            "mariadb" => Self::MariaDb,
            _ => return Err(TestError::parse_error(format!("Unknown MySQL version `{}`", s))),
        };

        Ok(version)
    }
}

impl ToString for MySqlVersion {
    fn to_string(&self) -> String {
        match self {
            MySqlVersion::V5_6 => "5.6",
            MySqlVersion::V5_7 => "5.7",
            MySqlVersion::V8 => "8",
            MySqlVersion::MariaDb => "mariadb",
        }
        .to_owned()
    }
}

fn mysql_capabilities() -> Vec<ConnectorCapability> {
    let dm_connector = MySqlDatamodelConnector::new(Default::default());
    dm_connector.capabilities().to_owned()
}
