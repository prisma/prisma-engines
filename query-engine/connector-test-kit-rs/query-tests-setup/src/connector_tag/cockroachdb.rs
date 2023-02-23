use super::*;
use crate::datamodel_rendering::SqlDatamodelRenderer;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CockroachDbConnectorTag {
    capabilities: Vec<ConnectorCapability>,
    version: CockroachDbVersion,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CockroachDbVersion {
    V222,
    V221,
}

impl fmt::Display for CockroachDbVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            CockroachDbVersion::V221 if is_ci => {
                format!("postgresql://prisma@test-db-cockroachdb:26257/{database}?schema={database}")
            }
            CockroachDbVersion::V222 if is_ci => {
                format!("postgresql://prisma@test-db-cockroachdb:26259/{database}?schema={database}")
            }
            CockroachDbVersion::V221 => {
                format!("postgresql://prisma@127.0.0.1:26257/{database}?schema={database}")
            }
            CockroachDbVersion::V222 => {
                format!("postgresql://prisma@127.0.0.1:26259/{database}?schema={database}")
            }
        }
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.to_string();
        ("cockroachdb".to_owned(), Some(version))
    }

    fn is_versioned(&self) -> bool {
        true
    }
}

impl CockroachDbConnectorTag {
    #[track_caller]
    pub fn new(version: Option<&str>) -> Self {
        let version = match version {
            Some("22.2") => CockroachDbVersion::V222,
            Some("22.1") | None => CockroachDbVersion::V221,
            _ => panic!("Unsupported CockroachDB Version: {:?}", version),
        };

        Self {
            capabilities: cockroachdb_capabilities(),
            version,
        }
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                version: CockroachDbVersion::V221,
                capabilities: cockroachdb_capabilities(),
            },
            Self {
                version: CockroachDbVersion::V222,
                capabilities: cockroachdb_capabilities(),
            },
        ]
    }
}

fn cockroachdb_capabilities() -> Vec<ConnectorCapability> {
    psl::builtin_connectors::COCKROACH.capabilities().to_owned()
}
