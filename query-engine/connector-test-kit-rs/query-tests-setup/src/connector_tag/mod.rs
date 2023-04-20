mod cockroachdb;
mod mongodb;
mod mysql;
mod postgres;
mod sql_server;
mod sqlite;
mod tidb;
mod vitess;

pub use mongodb::*;
pub use mysql::*;
pub use postgres::*;
pub use sql_server::*;
pub use sqlite::*;
pub use tidb::*;
pub use vitess::*;

use crate::{datamodel_rendering::DatamodelRenderer, TestError, CONFIG};
use cockroachdb::*;
use enum_dispatch::enum_dispatch;
use psl::datamodel_connector::ConnectorCapability;
use std::{convert::TryFrom, fmt};

#[enum_dispatch]
pub trait ConnectorTagInterface {
    /// The name of the datamodel provider for this connector.
    /// Must match valid datamodel provider strings.
    fn datamodel_provider(&self) -> &'static str;

    /// Returns the renderer to be used for templating the datamodel (the models portion).
    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer>;

    /// The connection string to use to connect to the test database and version.
    /// - `test_database` is the database to connect to, which is an implementation detail of the
    ///   implementing connector, like a file or a schema.
    /// - `is_ci` signals whether or not the test run is done on CI or not. May be important if local
    ///   test run connection strings and CI connection strings differ because of networking.
    fn connection_string(
        &self,
        test_database: &str,
        is_ci: bool,
        is_multi_schema: bool,
        isolation_level: Option<&'static str>,
    ) -> String;

    /// Capabilities of the implementing connector.
    fn capabilities(&self) -> &[ConnectorCapability];

    /// Serialization of the connector. Expected to return `(tag_name, version)`.
    /// Todo: Think of something better.
    fn as_parse_pair(&self) -> (String, Option<String>);

    /// Must return `true` if the connector family is versioned (e.g. Postgres9, Postgres10, ...), false otherwise.
    fn is_versioned(&self) -> bool;

    /// Defines where relational constraints are handled:
    ///   - "prisma" is handled in the Query Engine core
    ///   - "foreignKeys" lets the database handle them
    fn relation_mode(&self) -> &'static str {
        "foreignKeys"
    }
}

#[enum_dispatch(ConnectorTagInterface)]
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectorTag {
    SqlServer(SqlServerConnectorTag),
    Postgres(PostgresConnectorTag),
    MySql(MySqlConnectorTag),
    MongoDb(MongoDbConnectorTag),
    Sqlite(SqliteConnectorTag),
    Vitess(VitessConnectorTag),
    TiDB(TiDBConnectorTag),
    Cockroach(CockroachDbConnectorTag),
}

#[derive(Debug, Clone)]
pub enum ConnectorVersion {
    SqlServer(Option<SqlServerVersion>),
    Postgres(Option<PostgresVersion>),
    MySql(Option<MySqlVersion>),
    MongoDb(Option<MongoDbVersion>),
    Sqlite,
    CockroachDb,
    Vitess(Option<VitessVersion>),
    TiDB,
}

impl From<&ConnectorTag> for ConnectorVersion {
    fn from(connector: &ConnectorTag) -> Self {
        match connector {
            ConnectorTag::SqlServer(c) => ConnectorVersion::SqlServer(c.version()),
            ConnectorTag::Postgres(c) => ConnectorVersion::Postgres(c.version()),
            ConnectorTag::MySql(c) => ConnectorVersion::MySql(c.version()),
            ConnectorTag::MongoDb(c) => ConnectorVersion::MongoDb(c.version()),
            ConnectorTag::Sqlite(_) => ConnectorVersion::Sqlite,
            ConnectorTag::Cockroach(_) => ConnectorVersion::CockroachDb,
            ConnectorTag::Vitess(c) => ConnectorVersion::Vitess(c.version()),
            ConnectorTag::TiDB(_) => ConnectorVersion::TiDB,
        }
    }
}

impl fmt::Display for ConnectorTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printable = match self {
            Self::SqlServer(_) => "SQL Server",
            Self::Postgres(_) => "PostgreSQL",
            Self::MySql(_) => "MySQL",
            Self::MongoDb(_) => "MongoDB",
            Self::Sqlite(_) => "SQLite",
            Self::Vitess(_) => "Vitess",
            Self::Cockroach(_) => "CockroachDB",
            Self::TiDB(_) => "TiDB",
        };

        write!(f, "{printable}")
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
            Self::CockroachDb => "CockroachDB".to_string(),
            Self::TiDB => "TiDB".to_string(),
        };

        write!(f, "{printable}")
    }
}

impl ConnectorTag {
    /// Returns all possible connector tags.
    pub fn all() -> Vec<Self> {
        SqlServerConnectorTag::all()
            .into_iter()
            .map(Self::SqlServer)
            .chain(PostgresConnectorTag::all().into_iter().map(Self::Postgres))
            .chain(MySqlConnectorTag::all().into_iter().map(Self::MySql))
            .chain(MongoDbConnectorTag::all().into_iter().map(Self::MongoDb))
            .chain(SqliteConnectorTag::all().into_iter().map(Self::Sqlite))
            .chain(CockroachDbConnectorTag::all().into_iter().map(Self::Cockroach))
            .chain(TiDBConnectorTag::all().into_iter().map(Self::TiDB))
            .collect()
    }

    /// Determines whether or not a test should run for the given enabled connectors and capabilities
    /// a connector is required to have.
    pub(crate) fn should_run(
        only: &[(&str, Option<&str>)],
        exclude: &[(&str, Option<&str>)],
        capabilities: &[ConnectorCapability],
    ) -> bool {
        let connector = CONFIG.test_connector_tag().unwrap();

        if !capabilities.is_empty() && !capabilities.iter().all(|cap| connector.capabilities().contains(cap)) {
            println!("Connector excluded. Missing required capability.");
            return false;
        }

        if !only.is_empty() {
            return only
                .iter()
                .any(|only| ConnectorTag::try_from(*only).unwrap() == connector);
        }

        if exclude
            .iter()
            .any(|excl| ConnectorTag::try_from(*excl).unwrap() == connector)
        {
            println!("Connector excluded. Skipping test.");
            return false;
        }

        // FIXME: This skips vitess unless explicitly opted in. Replace with `true` when fixing
        // https://github.com/prisma/client-planning/issues/332
        !matches!(connector, ConnectorTag::Vitess(_))
    }
}

impl TryFrom<(&str, Option<&str>)> for ConnectorTag {
    type Error = TestError;

    #[track_caller]
    fn try_from(value: (&str, Option<&str>)) -> Result<Self, Self::Error> {
        let (connector, version) = value;

        let tag = match connector.to_lowercase().as_str() {
            "sqlite" => Self::Sqlite(SqliteConnectorTag::new()),
            "sqlserver" => Self::SqlServer(SqlServerConnectorTag::new(version)?),
            "cockroachdb" => Self::Cockroach(CockroachDbConnectorTag::new(version)?),
            "postgres" => Self::Postgres(PostgresConnectorTag::new(version)?),
            "tidb" => Self::TiDB(TiDBConnectorTag::new()),
            "mysql" => Self::MySql(MySqlConnectorTag::new(version)?),
            "mongodb" => Self::MongoDb(MongoDbConnectorTag::new(version)?),
            "vitess" => Self::Vitess(VitessConnectorTag::new(version)?),
            _ => return Err(TestError::parse_error(format!("Unknown connector tag `{connector}`"))),
        };

        Ok(tag)
    }
}
