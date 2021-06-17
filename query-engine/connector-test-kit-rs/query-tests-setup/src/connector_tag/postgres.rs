use datamodel_connector::Connector;
use sql_datamodel_connector::PostgresDatamodelConnector;

use crate::{datamodel_rendering::SqlDatamodelRenderer, TestError, TestResult};

use super::*;

#[derive(Debug, Default, Clone)]
pub struct PostgresConnectorTag {
    version: Option<PostgresVersion>,
    capabilities: Vec<ConnectorCapability>,
}

impl ConnectorTagInterface for PostgresConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "postgres"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(&self, database: &str, is_ci: bool) -> String {
        match self.version {
            Some(PostgresVersion::V9) if is_ci => format!(
                "postgresql://postgres:prisma@test-db-postgres-9:5432/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::V10) if is_ci => format!(
                "postgresql://postgres:prisma@test-db-postgres-10:5432/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::V11) if is_ci => format!(
                "postgresql://postgres:prisma@test-db-postgres-11:5432/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::V12) if is_ci => format!(
                "postgresql://postgres:prisma@test-db-postgres-12:5432/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::V13) if is_ci => format!(
                "postgresql://postgres:prisma@test-db-postgres-13:5432/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::PgBouncer) if is_ci => format!(
                "postgresql://postgres:prisma@test-db-pgbouncer:6432/db?schema={}&connection_limit=1&pgbouncer=true",
                database
            ),
            Some(PostgresVersion::V9) => format!(
                "postgresql://postgres:prisma@127.0.0.1:5431/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::V10) => format!(
                "postgresql://postgres:prisma@127.0.0.1:5432/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::V11) => format!(
                "postgresql://postgres:prisma@127.0.0.1:5433/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::V12) => format!(
                "postgresql://postgres:prisma@127.0.0.1:5434/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::V13) => format!(
                "postgresql://postgres:prisma@127.0.0.1:5434/db?schema={}&connection_limit=1",
                database
            ),
            Some(PostgresVersion::PgBouncer) => format!(
                "postgresql://postgres:prisma@127.0.0.1:6432/db?schema={}&connection_limit=1&pgbouncer=true",
                database
            ),

            None => unreachable!("A versioned connector must have a concrete version to run."),
        }
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.as_ref().map(ToString::to_string);
        ("postgres".to_owned(), version)
    }

    fn is_versioned(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostgresVersion {
    V9,
    V10,
    V11,
    V12,
    V13,
    PgBouncer,
}

impl PostgresConnectorTag {
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(PostgresVersion::try_from(v)?),
            None => None,
        };

        Ok(Self {
            version,
            capabilities: postgres_capabilities(),
        })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        let capabilities = postgres_capabilities();
        vec![
            Self {
                version: Some(PostgresVersion::V9),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(PostgresVersion::V10),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(PostgresVersion::V11),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(PostgresVersion::V12),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(PostgresVersion::V13),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(PostgresVersion::PgBouncer),
                capabilities,
            },
        ]
    }
}

impl PartialEq for PostgresConnectorTag {
    fn eq(&self, other: &Self) -> bool {
        match (self.version, other.version) {
            (None, None) | (Some(_), None) | (None, Some(_)) => true,
            (Some(v1), Some(v2)) => v1 == v2,
        }
    }
}

impl TryFrom<&str> for PostgresVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "9" => Self::V9,
            "10" => Self::V10,
            "11" => Self::V11,
            "12" => Self::V12,
            "13" => Self::V13,
            "pgbouncer" => Self::PgBouncer,
            _ => return Err(TestError::parse_error(format!("Unknown Postgres version `{}`", s))),
        };

        Ok(version)
    }
}

impl ToString for PostgresVersion {
    fn to_string(&self) -> String {
        match self {
            PostgresVersion::V9 => "9",
            PostgresVersion::V10 => "10",
            PostgresVersion::V11 => "11",
            PostgresVersion::V12 => "12",
            PostgresVersion::V13 => "13",
            PostgresVersion::PgBouncer => "pgbouncer",
        }
        .to_owned()
    }
}

fn postgres_capabilities() -> Vec<ConnectorCapability> {
    let dm_connector = PostgresDatamodelConnector::new();
    dm_connector.capabilities().to_owned()
}
