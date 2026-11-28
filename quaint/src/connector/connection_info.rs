#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]
#![cfg_attr(not(target_arch = "wasm32"), allow(clippy::large_enum_variant))]

#[cfg(any(
    feature = "sqlite-native",
    feature = "mysql-native",
    feature = "kingbase-mysql-native",
    feature = "kingbase-oracle-native",
    feature = "postgresql-native",
    feature = "mssql-native"
))]
use crate::error::{Error, ErrorKind};
use std::{borrow::Cow, fmt};
#[cfg(any(
    feature = "sqlite-native",
    feature = "mysql-native",
    feature = "kingbase-mysql-native",
    feature = "kingbase-oracle-native",
    feature = "postgresql-native",
    feature = "mssql-native"
))]
use url::Url;

#[cfg(feature = "kingbase-mysql-native")]
use crate::connector::KingbaseMysqlUrl;
#[cfg(feature = "kingbase-oracle-native")]
use crate::connector::KingbaseOracleUrl;
#[cfg(feature = "mssql-native")]
use crate::connector::MssqlUrl;
#[cfg(feature = "mysql-native")]
use crate::connector::MysqlUrl;
#[cfg(feature = "postgresql-native")]
use crate::connector::PostgresUrl;
#[cfg(feature = "sqlite-native")]
use crate::connector::SqliteParams;
#[cfg(feature = "sqlite-native")]
use std::convert::TryFrom;

use super::ExternalConnectionInfo;

/// KingbaseES uses the PostgreSQL wire protocol, whose prepared-statement
/// parameter count is represented by a signed 16-bit integer.
#[cfg(any(feature = "kingbase-mysql", feature = "kingbase-oracle"))]
const KINGBASE_MAX_BIND_VALUES: usize = i16::MAX as usize;

/// General information about a SQL connection.
#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", repr(transparent))]
pub enum ConnectionInfo {
    #[cfg(any(
        feature = "sqlite-native",
        feature = "mysql-native",
        feature = "kingbase-mysql-native",
        feature = "kingbase-oracle-native",
        feature = "postgresql-native",
        feature = "mssql-native"
    ))]
    Native(NativeConnectionInfo),
    External(ExternalConnectionInfo),
}

impl ConnectionInfo {
    /// Parse `ConnectionInfo` out from an SQL connection string.
    ///
    /// Will fail if URI is invalid or the scheme points to an unsupported
    /// database.
    #[cfg(any(
        feature = "sqlite-native",
        feature = "mysql-native",
        feature = "kingbase-mysql-native",
        feature = "kingbase-oracle-native",
        feature = "postgresql-native",
        feature = "mssql-native"
    ))]
    pub fn from_url(url_str: &str) -> crate::Result<Self> {
        let url_result: Result<Url, _> = url_str.parse();

        // Non-URL database strings are interpreted as SQLite file paths.
        match url_str {
            #[cfg(feature = "sqlite-native")]
            s if s.starts_with("file") => {
                if url_result.is_err() {
                    let params = SqliteParams::try_from(s)?;

                    return Ok(ConnectionInfo::Native(NativeConnectionInfo::Sqlite {
                        file_path: params.file_path,
                        db_name: params.db_name,
                    }));
                }
            }
            #[cfg(feature = "mssql-native")]
            s if s.starts_with("jdbc:sqlserver") || s.starts_with("sqlserver") => {
                return Ok(ConnectionInfo::Native(NativeConnectionInfo::Mssql(MssqlUrl::new(
                    url_str,
                )?)));
            }
            _ => (),
        }

        let url = url_result?;

        let sql_family = SqlFamily::from_scheme(url.scheme()).ok_or_else(|| {
            let kind =
                ErrorKind::DatabaseUrlIsInvalid(format!("{} is not a supported database URL scheme.", url.scheme()));

            Error::builder(kind).build()
        })?;

        #[cfg(feature = "kingbase-mysql-native")]
        if matches!(url.scheme(), "kingbase" | "kingbase-mysql") {
            return Ok(ConnectionInfo::Native(NativeConnectionInfo::KingbaseMysql(
                KingbaseMysqlUrl::new(url)?,
            )));
        }

        #[cfg(feature = "kingbase-oracle-native")]
        if url.scheme() == "kingbase-oracle" {
            return Ok(ConnectionInfo::Native(NativeConnectionInfo::KingbaseOracle(
                KingbaseOracleUrl::new(url)?,
            )));
        }

        match sql_family {
            #[cfg(feature = "mysql-native")]
            SqlFamily::Mysql => Ok(ConnectionInfo::Native(NativeConnectionInfo::Mysql(MysqlUrl::new(url)?))),
            #[cfg(feature = "sqlite-native")]
            SqlFamily::Sqlite => {
                let params = SqliteParams::try_from(url_str)?;

                Ok(ConnectionInfo::Native(NativeConnectionInfo::Sqlite {
                    file_path: params.file_path,
                    db_name: params.db_name,
                }))
            }
            #[cfg(feature = "postgresql-native")]
            SqlFamily::Postgres => Ok(ConnectionInfo::Native(NativeConnectionInfo::Postgres(
                super::PostgresUrl::new_native(url)?,
            ))),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }

    /// The provided database name. This will be `None` on SQLite.
    pub fn dbname(&self) -> Option<Cow<'_, str>> {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql-native")]
                NativeConnectionInfo::Postgres(url) => Some(url.dbname()),
                #[cfg(feature = "mysql-native")]
                NativeConnectionInfo::Mysql(url) => url.dbname().map(Cow::Borrowed),
                #[cfg(feature = "kingbase-mysql-native")]
                NativeConnectionInfo::KingbaseMysql(url) => url.dbname().map(Cow::Borrowed),
                #[cfg(feature = "kingbase-oracle-native")]
                NativeConnectionInfo::KingbaseOracle(url) => url.dbname().map(Cow::Borrowed),
                #[cfg(feature = "mssql-native")]
                NativeConnectionInfo::Mssql(url) => Some(Cow::Borrowed(url.dbname())),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => None,
            },
            ConnectionInfo::External(_) => None,
        }
    }

    /// This is what item names are prefixed with in queries.
    ///
    /// - In SQLite, this is the schema name that the database file was attached as.
    /// - In Postgres, it is the selected schema inside the current database.
    /// - In MySQL, it is the database name (may be empty for Vitess).
    pub fn schema_name(&self) -> Option<&str> {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql-native")]
                NativeConnectionInfo::Postgres(url) => Some(url.schema()),
                #[cfg(feature = "mysql-native")]
                NativeConnectionInfo::Mysql(url) => url.dbname(),
                #[cfg(feature = "kingbase-mysql-native")]
                NativeConnectionInfo::KingbaseMysql(url) => {
                    Some(url.schema().unwrap_or(crate::connector::DEFAULT_KINGBASE_MYSQL_SCHEMA))
                }
                #[cfg(feature = "kingbase-oracle-native")]
                NativeConnectionInfo::KingbaseOracle(url) => {
                    Some(url.schema().unwrap_or(crate::connector::DEFAULT_KINGBASE_ORACLE_SCHEMA))
                }
                #[cfg(feature = "mssql-native")]
                NativeConnectionInfo::Mssql(url) => Some(url.schema()),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::Sqlite { db_name, .. } => Some(db_name),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::InMemorySqlite { db_name } => Some(db_name),
            },
            ConnectionInfo::External(info) => info.schema_name.as_deref(),
        }
    }

    /// The provided database host. This will be `"localhost"` on SQLite.
    pub fn host(&self) -> &str {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql-native")]
                NativeConnectionInfo::Postgres(url) => url.host(),
                #[cfg(feature = "mysql-native")]
                NativeConnectionInfo::Mysql(url) => url.host(),
                #[cfg(feature = "kingbase-mysql-native")]
                NativeConnectionInfo::KingbaseMysql(url) => url.host(),
                #[cfg(feature = "kingbase-oracle-native")]
                NativeConnectionInfo::KingbaseOracle(url) => url.host(),
                #[cfg(feature = "mssql-native")]
                NativeConnectionInfo::Mssql(url) => url.host(),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => "localhost",
            },
            ConnectionInfo::External(_) => "external",
        }
    }

    /// The provided database user name. This will be `None` on SQLite.
    pub fn username(&self) -> Option<Cow<'_, str>> {
        // TODO: why do some of the native `.username()` methods return an `Option<&str>` and others a `Cow<str>`?
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql-native")]
                NativeConnectionInfo::Postgres(url) => Some(url.username()),
                #[cfg(feature = "mysql-native")]
                NativeConnectionInfo::Mysql(url) => Some(url.username()),
                #[cfg(feature = "kingbase-mysql-native")]
                NativeConnectionInfo::KingbaseMysql(url) => Some(Cow::Borrowed(url.username())),
                #[cfg(feature = "kingbase-oracle-native")]
                NativeConnectionInfo::KingbaseOracle(url) => Some(Cow::Borrowed(url.username())),
                #[cfg(feature = "mssql-native")]
                NativeConnectionInfo::Mssql(url) => url.username().map(Cow::from),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => None,
            },
            ConnectionInfo::External(_) => None,
        }
    }

    /// The database file for SQLite, otherwise `None`.
    pub fn file_path(&self) -> Option<&str> {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql-native")]
                NativeConnectionInfo::Postgres(_) => None,
                #[cfg(feature = "mysql-native")]
                NativeConnectionInfo::Mysql(_) => None,
                #[cfg(feature = "kingbase-mysql-native")]
                NativeConnectionInfo::KingbaseMysql(_) => None,
                #[cfg(feature = "kingbase-oracle-native")]
                NativeConnectionInfo::KingbaseOracle(_) => None,
                #[cfg(feature = "mssql-native")]
                NativeConnectionInfo::Mssql(_) => None,
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::Sqlite { file_path, .. } => Some(file_path),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::InMemorySqlite { .. } => None,
            },
            ConnectionInfo::External(_) => None,
        }
    }

    pub fn max_insert_rows(&self) -> Option<usize> {
        self.sql_family().max_insert_rows()
    }

    pub fn max_bind_values(&self) -> usize {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            #[cfg(feature = "kingbase-mysql-native")]
            ConnectionInfo::Native(NativeConnectionInfo::KingbaseMysql(_)) => {
                self.sql_family().max_bind_values().min(KINGBASE_MAX_BIND_VALUES)
            }
            #[cfg(feature = "kingbase-oracle-native")]
            ConnectionInfo::Native(NativeConnectionInfo::KingbaseOracle(_)) => {
                self.sql_family().max_bind_values().min(KINGBASE_MAX_BIND_VALUES)
            }
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            #[allow(unreachable_patterns)]
            ConnectionInfo::Native(_) => self.sql_family().max_bind_values(),
            // Wasm connectors can override the default max bind values.
            #[cfg(feature = "kingbase-mysql")]
            ConnectionInfo::External(info) if info.is_kingbase_mysql() => info
                .max_bind_values
                .unwrap_or(self.sql_family().max_bind_values())
                .min(KINGBASE_MAX_BIND_VALUES),
            #[cfg(feature = "kingbase-oracle")]
            ConnectionInfo::External(info) if info.sql_family.is_kingbase_oracle() => info
                .max_bind_values
                .unwrap_or(self.sql_family().max_bind_values())
                .min(KINGBASE_MAX_BIND_VALUES),
            ConnectionInfo::External(info) => info.max_bind_values.unwrap_or(self.sql_family().max_bind_values()),
        }
    }

    /// The family of databases connected.
    pub fn sql_family(&self) -> SqlFamily {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql-native")]
                NativeConnectionInfo::Postgres(_) => SqlFamily::Postgres,
                #[cfg(feature = "mysql-native")]
                NativeConnectionInfo::Mysql(_) => SqlFamily::Mysql,
                #[cfg(feature = "kingbase-mysql-native")]
                NativeConnectionInfo::KingbaseMysql(_) => SqlFamily::Mysql,
                #[cfg(feature = "kingbase-oracle-native")]
                NativeConnectionInfo::KingbaseOracle(_) => SqlFamily::KingbaseOracle,
                #[cfg(feature = "mssql-native")]
                NativeConnectionInfo::Mssql(_) => SqlFamily::Mssql,
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => SqlFamily::Sqlite,
            },
            ConnectionInfo::External(info) => info.sql_family.to_owned(),
        }
    }

    /// Whether this connection uses KingbaseES in MySQL-compatible mode.
    pub fn is_kingbase_mysql(&self) -> bool {
        match self {
            #[cfg(feature = "kingbase-mysql-native")]
            ConnectionInfo::Native(NativeConnectionInfo::KingbaseMysql(_)) => true,
            ConnectionInfo::External(info) => info.is_kingbase_mysql(),
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(_) => false,
        }
    }

    /// Whether this connection uses KingbaseES in Oracle-compatible mode.
    pub fn is_kingbase_oracle(&self) -> bool {
        match self {
            #[cfg(feature = "kingbase-oracle-native")]
            ConnectionInfo::Native(NativeConnectionInfo::KingbaseOracle(_)) => true,
            #[cfg(feature = "kingbase-oracle")]
            ConnectionInfo::External(info) => info.sql_family.is_kingbase_oracle(),
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(_) => false,
            #[cfg(not(feature = "kingbase-oracle"))]
            ConnectionInfo::External(_) => false,
        }
    }

    /// The provided database port, if applicable.
    pub fn port(&self) -> Option<u16> {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql-native")]
                NativeConnectionInfo::Postgres(url) => Some(url.port()),
                #[cfg(feature = "mysql-native")]
                NativeConnectionInfo::Mysql(url) => Some(url.port()),
                #[cfg(feature = "kingbase-mysql-native")]
                NativeConnectionInfo::KingbaseMysql(url) => Some(url.port()),
                #[cfg(feature = "kingbase-oracle-native")]
                NativeConnectionInfo::KingbaseOracle(url) => Some(url.port()),
                #[cfg(feature = "mssql-native")]
                NativeConnectionInfo::Mssql(url) => Some(url.port()),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => None,
            },
            ConnectionInfo::External(_) => None,
        }
    }

    /// Whether the pgbouncer mode is enabled.
    pub fn pg_bouncer(&self) -> bool {
        match self {
            #[cfg(all(not(target_arch = "wasm32"), feature = "postgresql-native"))]
            ConnectionInfo::Native(NativeConnectionInfo::Postgres(PostgresUrl::Native(url))) => url.pg_bouncer(),
            #[cfg(all(not(target_arch = "wasm32"), feature = "kingbase-mysql-native"))]
            ConnectionInfo::Native(NativeConnectionInfo::KingbaseMysql(url)) => url.pg_bouncer(),
            #[cfg(all(not(target_arch = "wasm32"), feature = "kingbase-oracle-native"))]
            ConnectionInfo::Native(NativeConnectionInfo::KingbaseOracle(url)) => url.pg_bouncer(),
            _ => false,
        }
    }

    /// A string describing the database location, meant for error messages. It will be the host
    /// and port on MySQL/Postgres, and the file path on SQLite.
    pub fn database_location(&self) -> String {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql-native")]
                NativeConnectionInfo::Postgres(url) => format!("{}:{}", url.host(), url.port()),
                #[cfg(feature = "mysql-native")]
                NativeConnectionInfo::Mysql(url) => format!("{}:{}", url.host(), url.port()),
                #[cfg(feature = "kingbase-mysql-native")]
                NativeConnectionInfo::KingbaseMysql(url) => format!("{}:{}", url.host(), url.port()),
                #[cfg(feature = "kingbase-oracle-native")]
                NativeConnectionInfo::KingbaseOracle(url) => format!("{}:{}", url.host(), url.port()),
                #[cfg(feature = "mssql-native")]
                NativeConnectionInfo::Mssql(url) => format!("{}:{}", url.host(), url.port()),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::Sqlite { file_path, .. } => file_path.clone(),
                #[cfg(feature = "sqlite-native")]
                NativeConnectionInfo::InMemorySqlite { .. } => "in-memory".into(),
            },
            ConnectionInfo::External(_) => "external".into(),
        }
    }

    #[allow(unused_variables)]
    pub fn set_version(&mut self, version: Option<String>) {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(native) => native.set_version(version),
            ConnectionInfo::External(_) => (),
        }
    }

    pub fn version(&self) -> Option<&str> {
        match self {
            #[cfg(feature = "mysql-native")]
            ConnectionInfo::Native(NativeConnectionInfo::Mysql(m)) => m.version(),
            _ => None,
        }
    }

    pub fn as_native(&self) -> Option<&NativeConnectionInfo> {
        match self {
            #[cfg(any(
                feature = "sqlite-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "mssql-native"
            ))]
            ConnectionInfo::Native(native) => Some(native),
            _ => None,
        }
    }
}

/// General information about a SQL connection, provided by native Rust drivers.
#[derive(Debug, Clone)]
pub enum NativeConnectionInfo {
    /// A PostgreSQL connection URL.
    #[cfg(feature = "postgresql-native")]
    Postgres(PostgresUrl),
    /// A MySQL connection URL.
    #[cfg(feature = "mysql-native")]
    Mysql(MysqlUrl),
    /// A KingbaseES connection URL in MySQL-compatible mode.
    #[cfg(feature = "kingbase-mysql-native")]
    KingbaseMysql(KingbaseMysqlUrl),
    /// A KingbaseES connection URL in Oracle-compatible mode.
    #[cfg(feature = "kingbase-oracle-native")]
    KingbaseOracle(KingbaseOracleUrl),
    /// A SQL Server connection URL.
    #[cfg(feature = "mssql-native")]
    Mssql(MssqlUrl),
    /// A SQLite connection URL.
    #[cfg(feature = "sqlite-native")]
    Sqlite {
        /// The filesystem path of the SQLite database.
        file_path: String,
        /// The name the database is bound to - Always "main"
        db_name: String,
    },
    #[cfg(feature = "sqlite-native")]
    InMemorySqlite { db_name: String },
}

impl NativeConnectionInfo {
    #[allow(unused)]
    pub fn set_version(&mut self, version: Option<String>) {
        #[cfg(feature = "mysql-native")]
        if let NativeConnectionInfo::Mysql(c) = self {
            c.set_version(version);
        }
    }
}

/// One of the supported SQL variants.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SqlFamily {
    #[cfg(feature = "postgresql")]
    Postgres,
    #[cfg(feature = "mysql")]
    Mysql,
    #[cfg(feature = "kingbase-oracle")]
    KingbaseOracle,
    #[cfg(feature = "sqlite")]
    Sqlite,
    #[cfg(feature = "mssql")]
    Mssql,
}

impl SqlFamily {
    /// Get a string representation of the family.
    pub fn as_str(self) -> &'static str {
        match self {
            #[cfg(feature = "postgresql")]
            SqlFamily::Postgres => "postgresql",
            #[cfg(feature = "mysql")]
            SqlFamily::Mysql => "mysql",
            #[cfg(feature = "kingbase-oracle")]
            SqlFamily::KingbaseOracle => "kingbase-oracle",
            #[cfg(feature = "sqlite")]
            SqlFamily::Sqlite => "sqlite",
            #[cfg(feature = "mssql")]
            SqlFamily::Mssql => "mssql",
        }
    }

    /// Convert url scheme to an SqlFamily.
    pub fn from_scheme(url_scheme: &str) -> Option<Self> {
        match url_scheme {
            #[cfg(feature = "sqlite")]
            "file" => Some(SqlFamily::Sqlite),
            #[cfg(feature = "postgresql")]
            "postgres" | "postgresql" => Some(SqlFamily::Postgres),
            #[cfg(feature = "mysql")]
            "mysql" => Some(SqlFamily::Mysql),
            #[cfg(feature = "kingbase-mysql-native")]
            "kingbase" | "kingbase-mysql" => Some(SqlFamily::Mysql),
            #[cfg(feature = "kingbase-oracle")]
            "kingbase-oracle" => Some(SqlFamily::KingbaseOracle),
            _ => None,
        }
    }

    /// Get the default max rows for a batch insert.
    pub(crate) fn max_insert_rows(&self) -> Option<usize> {
        match self {
            #[cfg(feature = "postgresql")]
            SqlFamily::Postgres => None,
            #[cfg(feature = "mysql")]
            SqlFamily::Mysql => None,
            #[cfg(feature = "kingbase-oracle")]
            SqlFamily::KingbaseOracle => None,
            #[cfg(feature = "sqlite")]
            SqlFamily::Sqlite => Some(999),
            #[cfg(feature = "mssql")]
            SqlFamily::Mssql => Some(1000),
        }
    }

    /// Get the max number of bind parameters for a single query, which in targets other
    /// than Wasm can be controlled with the env var QUERY_BATCH_SIZE.
    #[cfg(any(
        feature = "sqlite-native",
        feature = "mysql-native",
        feature = "kingbase-mysql-native",
        feature = "kingbase-oracle-native",
        feature = "postgresql-native",
        feature = "mssql-native"
    ))]
    pub fn max_bind_values(&self) -> usize {
        use std::sync::OnceLock;
        static BATCH_SIZE_OVERRIDE: OnceLock<Option<usize>> = OnceLock::new();
        BATCH_SIZE_OVERRIDE
            .get_or_init(|| {
                std::env::var("QUERY_BATCH_SIZE")
                    .ok()
                    .map(|size| size.parse().expect("QUERY_BATCH_SIZE: not a valid size"))
            })
            .unwrap_or(self.default_max_bind_values())
    }

    /// Get the max number of bind parameters for a single query, in Wasm there's no
    /// environment, and we omit that knob.
    #[cfg(not(any(
        feature = "sqlite-native",
        feature = "mysql-native",
        feature = "kingbase-mysql-native",
        feature = "kingbase-oracle-native",
        feature = "postgresql-native",
        feature = "mssql-native"
    )))]
    pub fn max_bind_values(&self) -> usize {
        self.default_max_bind_values()
    }

    /// Get the default max number of bind parameters for a single query.
    pub fn default_max_bind_values(&self) -> usize {
        match self {
            #[cfg(feature = "postgresql")]
            SqlFamily::Postgres => 32766,
            #[cfg(feature = "mysql")]
            SqlFamily::Mysql => 65535,
            #[cfg(feature = "kingbase-oracle")]
            SqlFamily::KingbaseOracle => KINGBASE_MAX_BIND_VALUES,
            #[cfg(feature = "sqlite")]
            SqlFamily::Sqlite => 999,
            #[cfg(feature = "mssql")]
            SqlFamily::Mssql => 2098,
        }
    }

    /// Check if a family exists for the given scheme.
    pub fn scheme_is_supported(url_scheme: &str) -> bool {
        Self::from_scheme(url_scheme).is_some()
    }

    /// True, if family is PostgreSQL.
    #[cfg(feature = "postgresql")]
    pub fn is_postgres(&self) -> bool {
        matches!(self, SqlFamily::Postgres)
    }

    /// True, if family is PostgreSQL.
    #[cfg(not(feature = "postgresql"))]
    pub fn is_postgres(&self) -> bool {
        false
    }

    /// True, if family is MySQL.
    #[cfg(feature = "mysql")]
    pub fn is_mysql(&self) -> bool {
        matches!(self, SqlFamily::Mysql)
    }

    /// True, if family is MySQL.
    #[cfg(not(feature = "mysql"))]
    pub fn is_mysql(&self) -> bool {
        false
    }

    /// True if the family is KingbaseES in Oracle-compatible mode.
    #[cfg(feature = "kingbase-oracle")]
    pub fn is_kingbase_oracle(&self) -> bool {
        matches!(self, SqlFamily::KingbaseOracle)
    }

    /// True if Kingbase Oracle support is not compiled in.
    #[cfg(not(feature = "kingbase-oracle"))]
    pub fn is_kingbase_oracle(&self) -> bool {
        false
    }

    /// True, if family is SQLite.
    #[cfg(feature = "sqlite")]
    pub fn is_sqlite(&self) -> bool {
        matches!(self, SqlFamily::Sqlite)
    }

    /// True, if family is SQLite.
    #[cfg(not(feature = "sqlite"))]
    pub fn is_sqlite(&self) -> bool {
        false
    }

    /// True, if family is SQL Server.
    #[cfg(feature = "mssql")]
    pub fn is_mssql(&self) -> bool {
        matches!(self, SqlFamily::Mssql)
    }

    /// True, if family is SQL Server.
    #[cfg(not(feature = "mssql"))]
    pub fn is_mssql(&self) -> bool {
        false
    }
}

impl fmt::Display for SqlFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "sqlite-native",
        feature = "mysql-native",
        feature = "kingbase-mysql",
        feature = "kingbase-oracle"
    ))]
    use super::*;

    #[test]
    #[cfg(feature = "sqlite-native")]
    fn sqlite_connection_info_from_str_interprets_relative_path_correctly() {
        let conn_info = ConnectionInfo::from_url("file:dev.db").unwrap();

        #[allow(irrefutable_let_patterns)]
        if let ConnectionInfo::Native(NativeConnectionInfo::Sqlite { file_path, db_name: _ }) = conn_info {
            assert_eq!(file_path, "dev.db");
        } else {
            panic!("Wrong type of connection info, should be Sqlite");
        }
    }

    #[test]
    #[cfg(feature = "mysql-native")]
    fn mysql_connection_info_from_str() {
        let conn_info = ConnectionInfo::from_url("mysql://myuser:my%23pass%23word@lclhst:5432/mydb").unwrap();

        #[allow(irrefutable_let_patterns)]
        if let ConnectionInfo::Native(NativeConnectionInfo::Mysql(url)) = conn_info {
            assert_eq!(url.password().unwrap(), "my#pass#word");
            assert_eq!(url.host(), "lclhst");
            assert_eq!(url.username(), "myuser");
            assert_eq!(url.dbname(), Some("mydb"));
        } else {
            panic!("Wrong type of connection info, should be Mysql");
        }
    }

    #[test]
    #[cfg(feature = "mysql-native")]
    fn mysql_connection_info_from_str_without_db_name() {
        let conn_info = ConnectionInfo::from_url("mysql://myuser:my%23pass%23word@lclhst:5432").unwrap();

        #[allow(irrefutable_let_patterns)]
        if let ConnectionInfo::Native(NativeConnectionInfo::Mysql(url)) = conn_info {
            assert_eq!(url.password().unwrap(), "my#pass#word");
            assert_eq!(url.host(), "lclhst");
            assert_eq!(url.username(), "myuser");
            assert_eq!(url.dbname(), None);
        } else {
            panic!("Wrong type of connection info, should be Mysql");
        }
    }

    #[test]
    #[cfg(feature = "kingbase-mysql")]
    fn kingbase_mysql_external_connection_info_limits_bind_values() {
        for max_bind_values in [None, Some(65_535)] {
            let connection_info = ConnectionInfo::External(
                ExternalConnectionInfo::new(SqlFamily::Mysql, None, max_bind_values, true).with_kingbase_mysql(),
            );

            assert_eq!(connection_info.max_bind_values(), 32_767);
        }
    }

    #[test]
    #[cfg(feature = "kingbase-mysql-native")]
    fn kingbase_mysql_native_connection_info_limits_bind_values() {
        let connection_info = ConnectionInfo::from_url("kingbase-mysql://localhost/app").unwrap();

        assert_eq!(connection_info.max_bind_values(), 32_767);
    }

    #[test]
    #[cfg(feature = "kingbase-oracle-native")]
    fn kingbase_oracle_connection_info_keeps_an_independent_family() {
        let connection_info = ConnectionInfo::from_url("kingbase-oracle://user@localhost/app?schema=tenant").unwrap();

        assert_eq!(connection_info.sql_family(), SqlFamily::KingbaseOracle);
        assert!(connection_info.is_kingbase_oracle());
        assert!(!connection_info.is_kingbase_mysql());
        assert_eq!(connection_info.dbname().as_deref(), Some("app"));
        assert_eq!(connection_info.schema_name(), Some("tenant"));
        assert_eq!(connection_info.max_bind_values(), 32_767);
    }
}
