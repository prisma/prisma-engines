mod cockroachdb;
mod js;
mod mongodb;
mod mysql;
mod postgres;
mod sql_server;
mod sqlite;
mod vitess;

pub use mysql::MySqlVersion;

pub(crate) use cockroachdb::*;
pub(crate) use js::*;
pub(crate) use mongodb::*;
pub(crate) use mysql::*;
pub(crate) use postgres::*;
pub(crate) use sql_server::*;
pub(crate) use sqlite::*;
pub(crate) use vitess::*;

use crate::{datamodel_rendering::DatamodelRenderer, BoxFuture, TestError, CONFIG};
use psl::datamodel_connector::ConnectorCapabilities;
use std::{convert::TryFrom, fmt};

pub trait ConnectorTagInterface {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>>;

    /// The name of the datamodel provider for this connector.
    /// Must match valid datamodel provider strings.
    fn datamodel_provider(&self) -> &str;

    /// Returns the renderer to be used for templating the datamodel (the models portion).
    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer>;

    /// Capabilities of the implementing connector.
    fn capabilities(&self) -> ConnectorCapabilities;

    /// Defines where relational constraints are handled:
    ///   - "prisma" is handled in the Query Engine core
    ///   - "foreignKeys" lets the database handle them
    fn relation_mode(&self) -> &str {
        "foreignKeys"
    }
}

/// The connection string to use to connect to the test database and version.
/// - `test_database` is the database to connect to, which is an implementation detail of the
///   implementing connector, like a file or a schema.
/// - `is_ci` signals whether or not the test run is done on CI or not. May be important if local
///   test run connection strings and CI connection strings differ because of networking.
pub(crate) fn connection_string(
    version: &ConnectorVersion,
    database: &str,
    is_ci: bool,
    is_multi_schema: bool,
    isolation_level: Option<&'static str>,
) -> String {
    match version {
        ConnectorVersion::SqlServer(v) => {
            let database = if is_multi_schema {
                format!("database={database};schema=dbo")
            } else {
                format!("database=master;schema={database}")
            };

            let isolation_level = isolation_level.unwrap_or("READ UNCOMMITTED");

            match v {
            Some(SqlServerVersion::V2017) if is_ci => format!("sqlserver://test-db-sqlserver-2017:1433;{database};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}"),
            Some(SqlServerVersion::V2017) => format!("sqlserver://127.0.0.1:1434;{database};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}"),

            Some(SqlServerVersion::V2019) if is_ci => format!("sqlserver://test-db-sqlserver-2019:1433;{database};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}"),
            Some(SqlServerVersion::V2019) => format!("sqlserver://127.0.0.1:1433;{database};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}"),

            Some(SqlServerVersion::V2022) if is_ci => format!("sqlserver://test-db-sqlserver-2022:1433;{database};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}"),
            Some(SqlServerVersion::V2022) => format!("sqlserver://127.0.0.1:1435;{database};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel={isolation_level}"),

            None => unreachable!("A versioned connector must have a concrete version to run."),
        }
        }
        ConnectorVersion::Postgres(v) => {
            let database = if is_multi_schema {
                database.to_string()
            } else {
                format!("db?schema={database}")
            };

            match v {
                Some(PostgresVersion::V9) if is_ci => {
                    format!("postgresql://postgres:prisma@test-db-postgres-9:5432/{database}")
                }
                Some(PostgresVersion::V10) if is_ci => {
                    format!("postgresql://postgres:prisma@test-db-postgres-10:5432/{database}")
                }
                Some(PostgresVersion::V11) if is_ci => {
                    format!("postgresql://postgres:prisma@test-db-postgres-11:5432/{database}")
                }
                Some(PostgresVersion::V12) if is_ci => {
                    format!("postgresql://postgres:prisma@test-db-postgres-12:5432/{database}")
                }
                Some(PostgresVersion::V13) if is_ci => {
                    format!("postgresql://postgres:prisma@test-db-postgres-13:5432/{database}")
                }
                Some(PostgresVersion::V14) if is_ci => {
                    format!("postgresql://postgres:prisma@test-db-postgres-14:5432/{database}")
                }
                Some(PostgresVersion::V15) if is_ci => {
                    format!("postgresql://postgres:prisma@test-db-postgres-15:5432/{database}")
                }
                Some(PostgresVersion::PgBouncer) if is_ci => {
                    format!("postgresql://postgres:prisma@test-db-pgbouncer:6432/{database}&pgbouncer=true")
                }

                Some(PostgresVersion::V9) => format!("postgresql://postgres:prisma@127.0.0.1:5431/{database}"),
                Some(PostgresVersion::V10) => format!("postgresql://postgres:prisma@127.0.0.1:5432/{database}"),
                Some(PostgresVersion::V11) => format!("postgresql://postgres:prisma@127.0.0.1:5433/{database}"),
                Some(PostgresVersion::V12) => format!("postgresql://postgres:prisma@127.0.0.1:5434/{database}"),
                Some(PostgresVersion::V13) => format!("postgresql://postgres:prisma@127.0.0.1:5435/{database}"),
                Some(PostgresVersion::V14) => format!("postgresql://postgres:prisma@127.0.0.1:5437/{database}"),
                Some(PostgresVersion::V15) => format!("postgresql://postgres:prisma@127.0.0.1:5438/{database}"),
                Some(PostgresVersion::PgBouncer) => {
                    format!("postgresql://postgres:prisma@127.0.0.1:6432/db?{database}&pgbouncer=true")
                }

                None => unreachable!("A versioned connector must have a concrete version to run."),
            }
        }
        ConnectorVersion::MySql(v) => match v {
            Some(MySqlVersion::V5_6) if is_ci => format!("mysql://root:prisma@test-db-mysql-5-6:3306/{database}"),
            Some(MySqlVersion::V5_7) if is_ci => format!("mysql://root:prisma@test-db-mysql-5-7:3306/{database}"),
            Some(MySqlVersion::V8) if is_ci => format!("mysql://root:prisma@test-db-mysql-8:3306/{database}"),
            Some(MySqlVersion::MariaDb) if is_ci => {
                format!("mysql://root:prisma@test-db-mysql-mariadb:3306/{database}")
            }
            Some(MySqlVersion::V5_6) => format!("mysql://root:prisma@127.0.0.1:3309/{database}"),
            Some(MySqlVersion::V5_7) => format!("mysql://root:prisma@127.0.0.1:3306/{database}"),
            Some(MySqlVersion::V8) => format!("mysql://root:prisma@127.0.0.1:3307/{database}"),
            Some(MySqlVersion::MariaDb) => {
                format!("mysql://root:prisma@127.0.0.1:3308/{database}")
            }

            None => unreachable!("A versioned connector must have a concrete version to run."),
        },
        ConnectorVersion::MongoDb(v) => match v {
            Some(MongoDbVersion::V4_2) if is_ci => format!(
                "mongodb://prisma:prisma@test-db-mongodb-4-2:27016/{database}?authSource=admin&retryWrites=true"
            ),
            Some(MongoDbVersion::V4_2) => {
                format!("mongodb://prisma:prisma@127.0.0.1:27016/{database}?authSource=admin&retryWrites=true")
            }
            Some(MongoDbVersion::V4_4) if is_ci => format!(
                "mongodb://prisma:prisma@test-db-mongodb-4-4:27017/{database}?authSource=admin&retryWrites=true"
            ),
            Some(MongoDbVersion::V4_4) => {
                format!("mongodb://prisma:prisma@127.0.0.1:27017/{database}?authSource=admin&retryWrites=true")
            }
            Some(MongoDbVersion::V5) if is_ci => {
                format!("mongodb://prisma:prisma@test-db-mongodb-5:27018/{database}?authSource=admin&retryWrites=true")
            }
            Some(MongoDbVersion::V5) => {
                format!("mongodb://prisma:prisma@127.0.0.1:27018/{database}?authSource=admin&retryWrites=true")
            }
            None => unreachable!("A versioned connector must have a concrete version to run."),
        },
        ConnectorVersion::Sqlite => {
            let workspace_root = std::env::var("WORKSPACE_ROOT")
                .unwrap_or_else(|_| ".".to_owned())
                .trim_end_matches('/')
                .to_owned();

            format!("file://{workspace_root}/db/{database}.db")
        }
        ConnectorVersion::CockroachDb(v) => {
            // Use the same database and schema name for CockroachDB - unfortunately CockroachDB
            // can't handle 1 schema per test in a database well at this point in time.
            match v {
                Some(CockroachDbVersion::V221) if is_ci => {
                    format!("postgresql://prisma@test-db-cockroachdb-22-1:26257/{database}?schema={database}")
                }
                Some(CockroachDbVersion::V222) if is_ci => {
                    format!("postgresql://prisma@test-db-cockroachdb-22-2:26259/{database}?schema={database}")
                }
                Some(CockroachDbVersion::V231) if is_ci => {
                    format!("postgresql://prisma@test-db-cockroachdb-23-1:26260/{database}?schema={database}")
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
        ConnectorVersion::Vitess(Some(VitessVersion::V8_0)) => "mysql://root@localhost:33807/test".into(),
        ConnectorVersion::Vitess(None) => unreachable!("A versioned connector must have a concrete version to run."),
    }
}

pub type ConnectorTag = &'static (dyn ConnectorTagInterface + Send + Sync);

#[derive(Debug, Clone)]
pub enum ConnectorVersion {
    SqlServer(Option<SqlServerVersion>),
    Postgres(Option<PostgresVersion>),
    MySql(Option<MySqlVersion>),
    MongoDb(Option<MongoDbVersion>),
    Sqlite,
    CockroachDb(Option<CockroachDbVersion>),
    Vitess(Option<VitessVersion>),
}

impl ConnectorVersion {
    fn matches_pattern(&self, pat: &ConnectorVersion) -> bool {
        use ConnectorVersion::*;

        fn versions_match<T: PartialEq>(opt_a: &Option<T>, opt_b: &Option<T>) -> bool {
            match (opt_a, opt_b) {
                (None, None) | (None, Some(_)) | (Some(_), None) => true,
                (Some(a), Some(b)) => a == b,
            }
        }

        match (self, pat) {
            (SqlServer(a), SqlServer(b)) => versions_match(a, b),
            (Postgres(a), Postgres(b)) => versions_match(a, b),
            (MySql(a), MySql(b)) => versions_match(a, b),
            (MongoDb(a), MongoDb(b)) => versions_match(a, b),
            (CockroachDb(a), CockroachDb(b)) => versions_match(a, b),
            (Vitess(a), Vitess(b)) => versions_match(a, b),
            (Sqlite, Sqlite) => true,

            (MongoDb(..), _)
            | (_, MongoDb(..))
            | (SqlServer(..), _)
            | (_, SqlServer(..))
            | (Sqlite, _)
            | (_, Sqlite)
            | (CockroachDb(..), _)
            | (_, CockroachDb(..))
            | (Vitess(..), _)
            | (_, Vitess(..))
            | (Postgres(..), _)
            | (_, Postgres(..)) => false,
        }
    }
}

impl fmt::Display for ConnectorVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printable = match self {
            Self::SqlServer(v) => match v {
                Some(v) => format!("SQL Server ({})", v.to_string()),
                None => "SQL Server (unknown)".to_string(),
            },
            Self::Postgres(v) => match v {
                Some(v) => format!("PostgreSQL ({})", v.to_string()),
                None => "PostgreSQL (unknown)".to_string(),
            },
            Self::MySql(v) => match v {
                Some(v) => format!("MySQL ({})", v.to_string()),
                None => "MySQL (unknown)".to_string(),
            },
            Self::MongoDb(v) => match v {
                Some(v) => format!("MongoDB ({})", v.to_string()),
                None => "MongoDB (unknown)".to_string(),
            },
            Self::Sqlite => "SQLite".to_string(),
            Self::Vitess(v) => match v {
                Some(v) => format!("Vitess ({v})"),
                None => "Vitess (unknown)".to_string(),
            },
            Self::CockroachDb(_) => "CockroachDB".to_string(),
        };

        write!(f, "{printable}")
    }
}

/// Determines whether or not a test should run for the given enabled connectors and capabilities
/// a connector is required to have.
pub(crate) fn should_run(
    only: &[(&str, Option<&str>)],
    exclude: &[(&str, Option<&str>)],
    capabilities: ConnectorCapabilities,
) -> bool {
    let (connector, version) = CONFIG.test_connector().unwrap();

    if !capabilities.is_empty() && !connector.capabilities().contains(capabilities) {
        println!("Connector excluded. Missing required capability.");
        return false;
    }

    // We skip tests that exclude JS driver adapters when an external test executor is configured.
    // A test that you only want to run with rust drivers can be annotated with exclude(JS)
    if CONFIG.external_test_executor().is_some() && exclude.iter().any(|excl| excl.0.to_uppercase() == "JS") {
        println!("Excluded test execution for JS driver adapters. Skipping test");
        return false;
    };
    // we consume the JS token to prevent it from being used in the following checks
    let exclude: Vec<_> = exclude.iter().filter(|excl| excl.0.to_uppercase() != "JS").collect();

    // We only run tests that include JS driver adapters when an external test executor is configured.
    // A test that you only want to run with js driver adapters can be annotated with only(JS)
    if CONFIG.external_test_executor().is_none() && only.iter().any(|incl| incl.0.to_uppercase() == "JS") {
        println!("Excluded test execution for rust driver adapters. Skipping test");
        return false;
    }
    // we consume the JS token to prevent it from being used in the following checks
    let only: Vec<_> = only.iter().filter(|incl| incl.0.to_uppercase() != "JS").collect();

    if !only.is_empty() {
        return only
            .iter()
            .any(|incl| ConnectorVersion::try_from(**incl).unwrap().matches_pattern(&version));
    }

    if exclude.iter().any(|excl| {
        ConnectorVersion::try_from(**excl)
            .map_or(false, |connector_version| connector_version.matches_pattern(&version))
    }) {
        println!("Connector excluded. Skipping test.");
        return false;
    }

    // FIXME: This skips vitess unless explicitly opted in. Replace with `true` when fixing
    // https://github.com/prisma/client-planning/issues/332
    !matches!(version, ConnectorVersion::Vitess(_))
}

impl TryFrom<(&str, Option<&str>)> for ConnectorVersion {
    type Error = TestError;

    #[track_caller]
    fn try_from((connector, version): (&str, Option<&str>)) -> Result<Self, Self::Error> {
        Ok(match connector.to_lowercase().as_str() {
            "sqlite" => ConnectorVersion::Sqlite,
            "sqlserver" => ConnectorVersion::SqlServer(version.map(SqlServerVersion::try_from).transpose()?),
            "cockroachdb" => ConnectorVersion::CockroachDb(version.map(CockroachDbVersion::try_from).transpose()?),
            "postgres" => ConnectorVersion::Postgres(version.map(PostgresVersion::try_from).transpose()?),
            "mysql" => ConnectorVersion::MySql(version.map(MySqlVersion::try_from).transpose()?),
            "mongodb" => ConnectorVersion::MongoDb(version.map(MongoDbVersion::try_from).transpose()?),
            "vitess" => ConnectorVersion::Vitess(version.map(|v| v.parse()).transpose()?),
            _ => return Err(TestError::parse_error(format!("Unknown connector tag `{connector}`"))),
        })
    }
}
