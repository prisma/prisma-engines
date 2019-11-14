use crate::{connector::{PostgresUrl, MysqlUrl, SqliteParams}, error::Error};
use url::Url;
use std::{convert::TryFrom, borrow::Cow, fmt};

/// General information about a SQL connection.
#[derive(Debug, Clone)]
pub enum ConnectionInfo {
    /// A PostgreSQL connection URL.
    Postgres(PostgresUrl),
    /// A MySQL connection URL.
    Mysql(MysqlUrl),
    /// A SQLite connection URL.
    Sqlite {
        /// The filesystem path of the SQLite database.
        file_path: String,
        /// The name the database is bound to (with `ATTACH DATABASE`), if available.
        db_name: Option<String>,
    },
}

impl ConnectionInfo {
    /// Parse `ConnectionInfo` out from an SQL connection string.
    ///
    /// Will fail if URI is invalid or the scheme points to an unsupported
    /// database.
    pub fn from_url(url_str: &str) -> crate::Result<Self> {
        let url_result: Result<Url, _> = url_str.parse();

        // Non-URL database strings are interpreted as SQLite file paths.
        if url_result.is_err() {
            let params = SqliteParams::try_from(url_str)?;
            return Ok(ConnectionInfo::Sqlite {
                file_path: params.file_path,
                db_name: None,
            });
        }

        let url = url_result?;

        let sql_family = SqlFamily::from_scheme(url.scheme()).ok_or_else(|| {
            Error::DatabaseUrlIsInvalid(format!("{} is not a supported database URL scheme.", url.scheme()))
        })?;

        match sql_family {
            SqlFamily::Mysql => Ok(ConnectionInfo::Mysql(MysqlUrl::new(url)?)),
            SqlFamily::Sqlite => {
                let params = SqliteParams::try_from(url_str)?;

                Ok(ConnectionInfo::Sqlite {
                    file_path: params.file_path,
                    db_name: params.db_name,
                })
            }
            SqlFamily::Postgres => Ok(ConnectionInfo::Postgres(PostgresUrl::new(url)?)),
        }
    }

    /// The provided database name. This will be `None` on SQLite.
    pub fn dbname(&self) -> Option<&str> {
        match self {
            ConnectionInfo::Postgres(url) => Some(url.dbname()),
            ConnectionInfo::Mysql(url) => Some(url.dbname()),
            ConnectionInfo::Sqlite { .. } => None,
        }
    }

    /// Will be the database name for MySQL and SQLite, pointing to an actual
    /// schema in PostgreSQL.
    pub fn schema_name(&self) -> Option<String> {
        match self {
            ConnectionInfo::Postgres(url) => Some(url.schema()),
            ConnectionInfo::Mysql(url) => Some(url.dbname().to_owned()),
            ConnectionInfo::Sqlite { db_name, .. } => db_name.as_ref().map(|s| s.to_owned()),
        }
    }

    /// The provided database host. This will be `"localhost"` on SQLite.
    pub fn host(&self) -> &str {
        match self {
            ConnectionInfo::Postgres(url) => url.host(),
            ConnectionInfo::Mysql(url) => url.host(),
            ConnectionInfo::Sqlite { .. } => "localhost",
        }
    }

    /// The provided database user name. This will be `None` on SQLite.
    pub fn username<'a>(&'a self) -> Option<Cow<'a, str>> {
        match self {
            ConnectionInfo::Postgres(url) => Some(url.username()),
            ConnectionInfo::Mysql(url) => Some(url.username()),
            ConnectionInfo::Sqlite { .. } => None,
        }
    }

    /// The database file for SQLite, otherwise `None`.
    pub fn file_path(&self) -> Option<&str> {
        match self {
            ConnectionInfo::Postgres(_) => None,
            ConnectionInfo::Mysql(_) => None,
            ConnectionInfo::Sqlite { file_path, .. } => Some(file_path),
        }
    }

    /// The family of databases connected.
    pub fn sql_family(&self) -> SqlFamily {
        match self {
            ConnectionInfo::Postgres(_) => SqlFamily::Postgres,
            ConnectionInfo::Mysql(_) => SqlFamily::Mysql,
            ConnectionInfo::Sqlite { .. } => SqlFamily::Sqlite,
        }
    }
}

/// One of the supported SQL variants.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SqlFamily {
    Postgres,
    Mysql,
    Sqlite,
}

impl SqlFamily {
    /// Get a string representation of the family.
    pub fn as_str(&self) -> &'static str {
        match self {
            SqlFamily::Postgres => "postgresql",
            SqlFamily::Mysql => "mysql",
            SqlFamily::Sqlite => "sqlite",
        }
    }

    /// Convert url scheme to an SqlFamily.
    pub fn from_scheme(url_scheme: &str) -> Option<Self> {
        match url_scheme {
            "sqlite" | "file" => Some(SqlFamily::Sqlite),
            "postgres" | "postgresql" => Some(SqlFamily::Postgres),
            "mysql" => Some(SqlFamily::Mysql),
            _ => None,
        }
    }

    /// Check if a family exists for the given scheme.
    pub fn scheme_is_supported(url_scheme: &str) -> bool {
        Self::from_scheme(url_scheme).is_some()
    }
}

impl fmt::Display for SqlFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
