use crate::{datamodel_rendering::SqlDatamodelRenderer, TestError, TestResult};

use super::*;

#[derive(Debug, Default, Clone)]
pub struct SqlServerConnectorTag {
    version: Option<SqlServerVersion>,
    capabilities: Vec<ConnectorCapability>,
}

impl ConnectorTagInterface for SqlServerConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "sqlserver"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(
        &self,
        database: &str,
        is_ci: bool,
        is_multi_schema: bool,
        isolation_level: Option<&'static str>,
    ) -> String {
        let database = if is_multi_schema {
            format!("database={};schema=dbo", database)
        } else {
            format!("database=master;schema={}", database)
        };

        let isolation_level = isolation_level.unwrap_or("READ UNCOMMITTED");

        match self.version {
            Some(SqlServerVersion::V2017) if is_ci => format!("sqlserver://test-db-sqlserver-2017:1433;{};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}", database),
            Some(SqlServerVersion::V2017) => format!("sqlserver://127.0.0.1:1434;{};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}", database),

            Some(SqlServerVersion::V2019) if is_ci => format!("sqlserver://test-db-sqlserver-2019:1433;{};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}", database),
            Some(SqlServerVersion::V2019) => format!("sqlserver://127.0.0.1:1433;{};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}", database),

            Some(SqlServerVersion::V2022) if is_ci => format!("sqlserver://test-db-sqlserver-2022:1433;{};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}", database),
            Some(SqlServerVersion::V2022) => format!("sqlserver://127.0.0.1:1435;{};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}", database),

            None => unreachable!("A versioned connector must have a concrete version to run."),
        }
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.as_ref().map(ToString::to_string);
        ("sqlserver".to_owned(), version)
    }

    fn is_versioned(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlServerVersion {
    V2017,
    V2019,
    V2022,
}

impl SqlServerConnectorTag {
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(SqlServerVersion::try_from(v)?),
            None => None,
        };

        Ok(Self {
            version,
            capabilities: sql_server_capabilities(),
        })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        let capabilities = sql_server_capabilities();

        vec![
            Self {
                version: Some(SqlServerVersion::V2017),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(SqlServerVersion::V2019),
                capabilities: capabilities.clone(),
            },
            Self {
                version: Some(SqlServerVersion::V2022),
                capabilities,
            },
        ]
    }

    /// Get a reference to the sql server connector tag's version.
    pub fn version(&self) -> Option<SqlServerVersion> {
        self.version
    }
}

impl PartialEq for SqlServerConnectorTag {
    fn eq(&self, other: &Self) -> bool {
        match (self.version, other.version) {
            (None, None) | (Some(_), None) | (None, Some(_)) => true,
            (Some(v1), Some(v2)) => v1 == v2,
        }
    }
}

impl TryFrom<&str> for SqlServerVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "2017" => Self::V2017,
            "2019" => Self::V2019,
            "2022" => Self::V2022,
            _ => return Err(TestError::parse_error(format!("Unknown SqlServer version `{}`", s))),
        };

        Ok(version)
    }
}

impl ToString for SqlServerVersion {
    fn to_string(&self) -> String {
        match self {
            SqlServerVersion::V2017 => "2017",
            SqlServerVersion::V2019 => "2019",
            SqlServerVersion::V2022 => "2022",
        }
        .to_owned()
    }
}

fn sql_server_capabilities() -> Vec<ConnectorCapability> {
    psl::builtin_connectors::MSSQL.capabilities().to_owned()
}
