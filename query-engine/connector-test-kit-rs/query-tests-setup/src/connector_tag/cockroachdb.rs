use super::*;
use crate::datamodel_rendering::SqlDatamodelRenderer;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CockroachDbConnectorTag {
    version: Option<CockroachDbVersion>,
    capabilities: Vec<ConnectorCapability>,
}

impl ConnectorTagInterface for CockroachDbConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "cockroachdb"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(&self, database: &str, is_ci: bool) -> String {
        // Use the same database and schema name for CockroachDB - unfortunately CockroachDB
        // can't handle 1 schema per test in a database well at this point in time.

        match self.version {
            Some(CockroachDbVersion::V22_1) if is_ci => format!(
                "postgresql://prisma@test-db-cockroachdb:26257/{0}?schema={0}",
                database
            ),
            Some(CockroachDbVersion::V22_1) => {
                format!(
                    "postgresql://prisma@127.0.0.1:26257/{0}?schema={0}",
                    database
                )
            }
            Some(CockroachDbVersion::V21_2) if is_ci => format!(
                "postgresql://prisma@test-db-cockroachdb:26258/{0}?schema={0}",
                database
            ),
            Some(CockroachDbVersion::V21_2) => {
                format!(
                    "postgresql://prisma@127.0.0.1:26258/{0}?schema={0}",
                    database
                )
            }
            None => unreachable!("A versioned connector must have a concrete version to run."),
        }
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.as_ref().map(ToString::to_string);
        ("cockroachdb".to_owned(), version)
    }

    fn is_versioned(&self) -> bool {
        true
    }

    fn requires_teardown(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CockroachDbVersion {
    V22_1,
    V21_2,
}

impl CockroachDbConnectorTag {
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(CockroachDbVersion::try_from(v)?),
            None => None,
        };

        Ok(Self {
            version,
            capabilities: cockroachdb_capabilities(),
        })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                version: Some(CockroachDbVersion::V22_1),
                capabilities: cockroachdb_capabilities(),
            },
            Self {
                version: Some(CockroachDbVersion::V21_2),
                capabilities: cockroachdb_capabilities(),
            },
        ]
    }

    /// Get a reference to the connector tag's version.
    pub fn version(&self) -> Option<CockroachDbVersion> {
        self.version
    }
}

impl PartialEq for CockroachDbConnectorTag {
    fn eq(&self, other: &Self) -> bool {
        match (self.version, other.version) {
            (None, None) | (Some(_), None) | (None, Some(_)) => true,
            (Some(v1), Some(v2)) => v1 == v2,
        }
    }
}

impl TryFrom<&str> for CockroachDbVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "22.1.0" => Self::V22_1,
            "21.2.0" => Self::V21_2,
            _ => return Err(TestError::parse_error(format!("Unknown CockroachDB version `{}`", s))),
        };

        Ok(version)
    }
}

impl ToString for CockroachDbVersion {
    fn to_string(&self) -> String {
        match self {
            CockroachDbVersion::V22_1 => "22.1.0",
            &CockroachDbVersion::V21_2 => "21.2.0",
        }
        .to_owned()
    }
}

fn cockroachdb_capabilities() -> Vec<ConnectorCapability> {
    sql_datamodel_connector::COCKROACH.capabilities().to_owned()
}
