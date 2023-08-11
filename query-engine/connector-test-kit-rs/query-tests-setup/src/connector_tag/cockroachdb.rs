use super::*;
use crate::{datamodel_rendering::SqlDatamodelRenderer, TestResult};
use psl::datamodel_connector::ConnectorCapabilities;

#[derive(Debug, Default, Clone)]
pub struct CockroachDbConnectorTag {
    version: Option<CockroachDbVersion>,
}

impl PartialEq for CockroachDbConnectorTag {
    fn eq(&self, other: &Self) -> bool {
        match (self.version, other.version) {
            (None, None) | (Some(_), None) | (None, Some(_)) => true,
            (Some(v1), Some(v2)) => v1 == v2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CockroachDbVersion {
    V231,
    V222,
    V221,
}

impl TryFrom<&str> for CockroachDbVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "22.1" => Self::V221,
            "22.2" => Self::V222,
            "23.1" => Self::V231,
            _ => return Err(TestError::parse_error(format!("Unknown CockroachDB version `{s}`"))),
        };

        Ok(version)
    }
}

impl fmt::Display for CockroachDbVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CockroachDbVersion::V231 => f.write_str("23.1"),
            CockroachDbVersion::V222 => f.write_str("22.2"),
            CockroachDbVersion::V221 => f.write_str("22.1"),
        }
    }
}

impl Default for CockroachDbVersion {
    fn default() -> Self {
        Self::V221
    }
}

impl ConnectorTagInterface for CockroachDbConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "cockroachdb"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(
        &self,
        database: &str,
        is_ci: bool,
        _is_multi_schema: bool,
        _: Option<&'static str>,
    ) -> String {
        // Use the same database and schema name for CockroachDB - unfortunately CockroachDB
        // can't handle 1 schema per test in a database well at this point in time.
        match self.version {
            Some(CockroachDbVersion::V221) if is_ci => {
                format!("postgresql://prisma@test-db-cockroachdb-22-1:26257/{database}?schema={database}")
            }
            Some(CockroachDbVersion::V222) if is_ci => {
                format!("postgresql://prisma@test-db-cockroachdb-22-2:26259/{database}?schema={database}")
            }
            Some(CockroachDbVersion::V231) if is_ci => {
                format!("postgresql://prisma@test-db-cockroachdb-22-2:26260/{database}?schema={database}")
            }
            Some(CockroachDbVersion::V221) => {
                format!("postgresql://prisma@127.0.0.1:26257/{database}?schema={database}")
            }
            Some(CockroachDbVersion::V222) => {
                format!("postgresql://prisma@127.0.0.1:26259/{database}?schema={database}")
            }
            Some(CockroachDbVersion::V231) => {
                format!("postgresql://prisma@127.0.0.1:26260/{database}?schema={database}")
            }

            None => unreachable!("A versioned connector must have a concrete version to run."),
        }
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::COCKROACH.capabilities()
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.as_ref().map(ToString::to_string);
        ("cockroachdb".to_owned(), version)
    }

    fn is_versioned(&self) -> bool {
        true
    }
}

impl CockroachDbConnectorTag {
    #[track_caller]
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(CockroachDbVersion::try_from(v)?),
            None => None,
        };

        Ok(Self { version })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                version: Some(CockroachDbVersion::V221),
            },
            Self {
                version: Some(CockroachDbVersion::V222),
            },
            Self {
                version: Some(CockroachDbVersion::V231),
            },
        ]
    }
}
