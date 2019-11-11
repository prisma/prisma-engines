use crate::SqlFamily;
use datamodel::{
    configuration::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    Source,
};
use quaint::{
    connector::{MysqlUrl, PostgresUrl, SqliteParams},
    error::Error as QuaintError,
};
use std::borrow::Cow;
use std::convert::TryFrom;
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
        db_name: Option<String>,
    },
}

impl ConnectionInfo {
    pub fn from_datasource(datasource: &dyn Source) -> Result<Self, QuaintError> {
        let url = &datasource.url().value;

        match datasource.connector_type() {
            c if c == MYSQL_SOURCE_NAME => Ok(ConnectionInfo::Mysql(MysqlUrl::new(url.parse()?)?)),
            c if c == POSTGRES_SOURCE_NAME => Ok(ConnectionInfo::Postgres(PostgresUrl::new(url.parse()?)?)),
            c if c == SQLITE_SOURCE_NAME => {
                let params = SqliteParams::try_from(url.as_str())?;
                Ok(ConnectionInfo::Sqlite {
                    file_path: params.file_path,
                    db_name: None,
                })
            }
            c => panic!("Unsuppored connector type for SQL connection: {}", c),
        }
    }

    pub fn from_url_str(url_str: &str) -> Result<Self, QuaintError> {
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
            QuaintError::DatabaseUrlIsInvalid(format!("{} is not a supported database URL scheme.", url.scheme()))
        })?;

        match sql_family {
            SqlFamily::Mysql => Ok(ConnectionInfo::Mysql(MysqlUrl::new(url)?)),
            SqlFamily::Sqlite => {
                let params = SqliteParams::try_from(url_str)?;
                Ok(ConnectionInfo::Sqlite {
                    file_path: params.file_path,
                    db_name: None,
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

    pub fn file_path(&self) -> Option<&str> {
        match self {
            ConnectionInfo::Postgres(_) => None,
            ConnectionInfo::Mysql(_) => None,
            ConnectionInfo::Sqlite { file_path, .. } => Some(file_path),
        }
    }

    pub fn sql_family(&self) -> SqlFamily {
        match self {
            ConnectionInfo::Postgres(_) => SqlFamily::Postgres,
            ConnectionInfo::Mysql(_) => SqlFamily::Mysql,
            ConnectionInfo::Sqlite { .. } => SqlFamily::Sqlite,
        }
    }
}
