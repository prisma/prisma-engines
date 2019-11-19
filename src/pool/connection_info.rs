use crate::{
    connector::{MysqlUrl, PostgresUrl, SqliteParams},
    error::Error,
};
use std::{borrow::Cow, convert::TryFrom, fmt};
use url::Url;

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
        db_name: String,
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
                db_name: params.db_name.clone(),
            });
        }

        let url = url_result?;

        let sql_family = SqlFamily::from_scheme(url.scheme()).ok_or_else(|| {
            Error::DatabaseUrlIsInvalid(format!(
                "{} is not a supported database URL scheme.",
                url.scheme()
            ))
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

    /// This is what item names are prefixed with in queries.
    ///
    /// - In SQLite, this is the schema name that the database file was attached as.
    /// - In Postgres, it is the selected schema inside the current database.
    /// - In MySQL, it is the database name.
    pub fn schema_name(&self) -> &str {
        match self {
            ConnectionInfo::Postgres(url) => url.schema(),
            ConnectionInfo::Mysql(url) => url.dbname(),
            ConnectionInfo::Sqlite { db_name, .. } => db_name,
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

    /// The provided database port, if applicable.
    pub fn port(&self) -> Option<u16> {
        match self {
            ConnectionInfo::Postgres(url) => Some(url.port()),
            ConnectionInfo::Mysql(url) => Some(url.port()),
            ConnectionInfo::Sqlite { .. } => None,
        }
    }

    /// A string describing the database location, meant for error messages. It will be the host
    /// and port on MySQL/Postgres, and the file path on SQLite.
    pub fn database_location(&self) -> String {
        match self {
            ConnectionInfo::Postgres(url) => format!("{}:{}", url.host(), url.port()),
            ConnectionInfo::Mysql(url) => format!("{}:{}", url.host(), url.port()),
            ConnectionInfo::Sqlite { file_path, .. } => file_path.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_connection_info_from_str_interprets_relative_path_correctly() {
        let conn_info = ConnectionInfo::from_url("file:dev.db").unwrap();

        match conn_info {
            ConnectionInfo::Sqlite {
                file_path,
                db_name: _,
            } => assert_eq!(file_path, "dev.db"),
            _ => panic!("wrong"),
        }
    }
}
