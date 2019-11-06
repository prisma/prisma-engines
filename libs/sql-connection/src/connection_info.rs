use quaint::connector::{MysqlUrl, PostgresUrl};
use std::borrow::Cow;

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
}
