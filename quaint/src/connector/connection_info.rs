#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]
#![cfg_attr(not(target_arch = "wasm32"), allow(clippy::large_enum_variant))]

use crate::error::{Error, ErrorKind};
use std::{borrow::Cow, fmt};
use url::Url;

#[cfg(feature = "mssql")]
use crate::connector::MssqlUrl;
#[cfg(feature = "mysql")]
use crate::connector::MysqlUrl;
#[cfg(feature = "postgresql")]
use crate::connector::PostgresUrl;
#[cfg(feature = "sqlite")]
use crate::connector::SqliteParams;
#[cfg(feature = "sqlite")]
use std::convert::TryFrom;

use super::ExternalConnectionInfo;

#[cfg(native)]
use super::NativeConnectionInfo;

/// General information about a SQL connection.
#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", repr(transparent))]
pub enum ConnectionInfo {
    #[cfg(not(target_arch = "wasm32"))]
    Native(NativeConnectionInfo),
    External(ExternalConnectionInfo),
}

impl ConnectionInfo {
    /// Parse `ConnectionInfo` out from an SQL connection string.
    ///
    /// Will fail if URI is invalid or the scheme points to an unsupported
    /// database.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_url(url_str: &str) -> crate::Result<Self> {
        let url_result: Result<Url, _> = url_str.parse();

        // Non-URL database strings are interpreted as SQLite file paths.
        match url_str {
            #[cfg(feature = "sqlite")]
            s if s.starts_with("file") => {
                if url_result.is_err() {
                    let params = SqliteParams::try_from(s)?;

                    return Ok(ConnectionInfo::Native(NativeConnectionInfo::Sqlite {
                        file_path: params.file_path,
                        db_name: params.db_name,
                    }));
                }
            }
            #[cfg(feature = "mssql")]
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

        match sql_family {
            #[cfg(feature = "mysql")]
            SqlFamily::Mysql => Ok(ConnectionInfo::Native(NativeConnectionInfo::Mysql(MysqlUrl::new(url)?))),
            #[cfg(feature = "sqlite")]
            SqlFamily::Sqlite => {
                let params = SqliteParams::try_from(url_str)?;

                Ok(ConnectionInfo::Native(NativeConnectionInfo::Sqlite {
                    file_path: params.file_path,
                    db_name: params.db_name,
                }))
            }
            #[cfg(feature = "postgresql")]
            SqlFamily::Postgres => Ok(ConnectionInfo::Native(NativeConnectionInfo::Postgres(
                PostgresUrl::new(url)?,
            ))),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }

    /// The provided database name. This will be `None` on SQLite.
    pub fn dbname(&self) -> Option<&str> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql")]
                NativeConnectionInfo::Postgres(url) => Some(url.dbname()),
                #[cfg(feature = "mysql")]
                NativeConnectionInfo::Mysql(url) => Some(url.dbname()),
                #[cfg(feature = "mssql")]
                NativeConnectionInfo::Mssql(url) => Some(url.dbname()),
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => None,
            },
            ConnectionInfo::External(_) => None,
        }
    }

    /// This is what item names are prefixed with in queries.
    ///
    /// - In SQLite, this is the schema name that the database file was attached as.
    /// - In Postgres, it is the selected schema inside the current database.
    /// - In MySQL, it is the database name.
    pub fn schema_name(&self) -> &str {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql")]
                NativeConnectionInfo::Postgres(url) => url.schema(),
                #[cfg(feature = "mysql")]
                NativeConnectionInfo::Mysql(url) => url.dbname(),
                #[cfg(feature = "mssql")]
                NativeConnectionInfo::Mssql(url) => url.schema(),
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::Sqlite { db_name, .. } => db_name,
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::InMemorySqlite { db_name } => db_name,
            },
            ConnectionInfo::External(info) => &info.schema_name,
        }
    }

    /// The provided database host. This will be `"localhost"` on SQLite.
    pub fn host(&self) -> &str {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql")]
                NativeConnectionInfo::Postgres(url) => url.host(),
                #[cfg(feature = "mysql")]
                NativeConnectionInfo::Mysql(url) => url.host(),
                #[cfg(feature = "mssql")]
                NativeConnectionInfo::Mssql(url) => url.host(),
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => "localhost",
            },
            ConnectionInfo::External(_) => "external",
        }
    }

    /// The provided database user name. This will be `None` on SQLite.
    pub fn username(&self) -> Option<Cow<str>> {
        // TODO: why do some of the native `.username()` methods return an `Option<&str>` and others a `Cow<str>`?
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql")]
                NativeConnectionInfo::Postgres(url) => Some(url.username()),
                #[cfg(feature = "mysql")]
                NativeConnectionInfo::Mysql(url) => Some(url.username()),
                #[cfg(feature = "mssql")]
                NativeConnectionInfo::Mssql(url) => url.username().map(Cow::from),
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => None,
            },
            ConnectionInfo::External(_) => None,
        }
    }

    /// The database file for SQLite, otherwise `None`.
    pub fn file_path(&self) -> Option<&str> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql")]
                NativeConnectionInfo::Postgres(_) => None,
                #[cfg(feature = "mysql")]
                NativeConnectionInfo::Mysql(_) => None,
                #[cfg(feature = "mssql")]
                NativeConnectionInfo::Mssql(_) => None,
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::Sqlite { file_path, .. } => Some(file_path),
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::InMemorySqlite { .. } => None,
            },
            ConnectionInfo::External(_) => None,
        }
    }

    /// The family of databases connected.
    pub fn sql_family(&self) -> SqlFamily {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql")]
                NativeConnectionInfo::Postgres(_) => SqlFamily::Postgres,
                #[cfg(feature = "mysql")]
                NativeConnectionInfo::Mysql(_) => SqlFamily::Mysql,
                #[cfg(feature = "mssql")]
                NativeConnectionInfo::Mssql(_) => SqlFamily::Mssql,
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => SqlFamily::Sqlite,
            },
            ConnectionInfo::External(info) => info.sql_family.to_owned(),
        }
    }

    /// The provided database port, if applicable.
    pub fn port(&self) -> Option<u16> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql")]
                NativeConnectionInfo::Postgres(url) => Some(url.port()),
                #[cfg(feature = "mysql")]
                NativeConnectionInfo::Mysql(url) => Some(url.port()),
                #[cfg(feature = "mssql")]
                NativeConnectionInfo::Mssql(url) => Some(url.port()),
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. } => None,
            },
            ConnectionInfo::External(_) => None,
        }
    }

    /// Whether the pgbouncer mode is enabled.
    pub fn pg_bouncer(&self) -> bool {
        match self {
            #[cfg(all(not(target_arch = "wasm32"), feature = "postgresql"))]
            ConnectionInfo::Native(NativeConnectionInfo::Postgres(url)) => url.pg_bouncer(),
            _ => false,
        }
    }

    /// A string describing the database location, meant for error messages. It will be the host
    /// and port on MySQL/Postgres, and the file path on SQLite.
    pub fn database_location(&self) -> String {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            ConnectionInfo::Native(info) => match info {
                #[cfg(feature = "postgresql")]
                NativeConnectionInfo::Postgres(url) => format!("{}:{}", url.host(), url.port()),
                #[cfg(feature = "mysql")]
                NativeConnectionInfo::Mysql(url) => format!("{}:{}", url.host(), url.port()),
                #[cfg(feature = "mssql")]
                NativeConnectionInfo::Mssql(url) => format!("{}:{}", url.host(), url.port()),
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::Sqlite { file_path, .. } => file_path.clone(),
                #[cfg(feature = "sqlite")]
                NativeConnectionInfo::InMemorySqlite { .. } => "in-memory".into(),
            },
            ConnectionInfo::External(_) => "external".into(),
        }
    }

    #[allow(unused_variables)]
    pub fn set_version(&mut self, version: Option<String>) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
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
}

/// One of the supported SQL variants.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SqlFamily {
    #[cfg(feature = "postgresql")]
    Postgres,
    #[cfg(feature = "mysql")]
    Mysql,
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
            _ => None,
        }
    }

    /// Get the default max rows for a batch insert.
    pub fn max_insert_rows(&self) -> Option<usize> {
        match self {
            #[cfg(feature = "postgresql")]
            SqlFamily::Postgres => None,
            #[cfg(feature = "mysql")]
            SqlFamily::Mysql => None,
            #[cfg(feature = "sqlite")]
            SqlFamily::Sqlite => Some(999),
            #[cfg(feature = "mssql")]
            SqlFamily::Mssql => Some(1000),
        }
    }

    /// Get the max number of bind parameters for a single query, which in targets other
    /// than Wasm can be controlled with the env var QUERY_BATCH_SIZE.
    #[cfg(not(target_arch = "wasm32"))]
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
    #[cfg(target_arch = "wasm32")]
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
            #[cfg(feature = "sqlite")]
            SqlFamily::Sqlite => 999,
            #[cfg(feature = "mssql")]
            SqlFamily::Mssql => 2099,
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
    #[cfg(any(feature = "sqlite", feature = "mysql"))]
    use super::*;

    #[test]
    #[cfg(feature = "sqlite")]
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
    #[cfg(feature = "mysql")]
    fn mysql_connection_info_from_str() {
        let conn_info = ConnectionInfo::from_url("mysql://myuser:my%23pass%23word@lclhst:5432/mydb").unwrap();

        #[allow(irrefutable_let_patterns)]
        if let ConnectionInfo::Native(NativeConnectionInfo::Mysql(url)) = conn_info {
            assert_eq!(url.password().unwrap(), "my#pass#word");
            assert_eq!(url.host(), "lclhst");
            assert_eq!(url.username(), "myuser");
            assert_eq!(url.dbname(), "mydb");
        } else {
            panic!("Wrong type of connection info, should be Mysql");
        }
    }
}
