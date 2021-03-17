use crate::{TestError, TestResult};

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct SqlServerConnectorTag {
    version: Option<SqlServerVersion>,
}

impl ConnectorTagInterface for SqlServerConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "sqlserver"
    }

    fn render_datamodel(&self, template: String) -> String {
        // Todo, pass through for now
        template
    }

    fn connection_string(&self, database: &str, is_ci: bool) -> String {
        match self.version {
            Some(SqlServerVersion::V2017) if is_ci => format!("sqlserver://test-db-mssql-2017:1433;database=master;schema={};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel=READ UNCOMMITTED", database),
            Some(SqlServerVersion::V2017) => format!("sqlserver://127.0.0.1:1434;database=master;schema={};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel=READ UNCOMMITTED", database),

            Some(SqlServerVersion::V2019) if is_ci => format!("sqlserver://test-db-mssql-2019:1433;database=master;schema={};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel=READ UNCOMMITTED", database),
            Some(SqlServerVersion::V2019) => format!("sqlserver://127.0.0.1:1433;database=master;schema={};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel=READ UNCOMMITTED", database),

            None => unreachable!("A versioned connector must have a concrete version to run."),
        }.to_string()
    }

    fn capabilities(&self) -> Vec<ConnectorCapability> {
        todo!()
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
}

impl SqlServerConnectorTag {
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(SqlServerVersion::try_from(v)?),
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
        }
        .to_owned()
    }
}
