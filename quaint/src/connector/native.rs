#[cfg(feature = "mssql")]
use crate::connector::MssqlUrl;
#[cfg(feature = "mysql")]
use crate::connector::MysqlUrl;
#[cfg(feature = "postgresql")]
use crate::connector::PostgresUrl;

/// General information about a SQL connection, provided by native Rust drivers.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub enum NativeConnectionInfo {
    /// A PostgreSQL connection URL.
    #[cfg(feature = "postgresql")]
    Postgres(PostgresUrl),
    /// A MySQL connection URL.
    #[cfg(feature = "mysql")]
    Mysql(MysqlUrl),
    /// A SQL Server connection URL.
    #[cfg(feature = "mssql")]
    Mssql(MssqlUrl),
    /// A SQLite connection URL.
    #[cfg(feature = "sqlite")]
    Sqlite {
        /// The filesystem path of the SQLite database.
        file_path: String,
        /// The name the database is bound to - Always "main"
        db_name: String,
    },
    #[cfg(feature = "sqlite")]
    InMemorySqlite { db_name: String },
}

impl NativeConnectionInfo {
    #[allow(unused)]
    pub fn set_version(&mut self, version: Option<String>) {
        #[cfg(feature = "mysql")]
        if let NativeConnectionInfo::Mysql(c) = self {
            c.set_version(version);
        }
    }
}
