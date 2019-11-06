use crate::{SqlFamily, ConnectionInfo, Mysql, Postgresql, SqlConnection, Sqlite, SyncSqlConnection};
use quaint::{ast::*, connector::ResultSet, error::Error as QuaintError};
use url::Url;

/// A connection to a supported SQL database. This is mainly useful to abstract the _construction_
/// of a SQL connection from a URL.
pub enum GenericSqlConnection {
    /// A PostgreSQL connection.
    Postgresql(Postgresql),
    /// A MySQL connection.
    Mysql(Mysql),
    /// A SQLite connection.
    Sqlite(Sqlite),
}

impl GenericSqlConnection {
    /// Create a pooled database connection. The `db_name` param is only used on SQLite if you want
    /// to provide a specific name for the bound database.
    pub fn new(url_str: &str, db_name: Option<&str>) -> Result<Self, QuaintError> {
        let url: Url = url_str.parse().or_else(|_err| format!("file://{}", url_str).parse())?;

        match SqlFamily::from_scheme(url.scheme()) {
            Some(SqlFamily::Postgres) => Ok(GenericSqlConnection::Postgresql(Postgresql::new(url)?)),
            Some(SqlFamily::Mysql) => Ok(GenericSqlConnection::Mysql(Mysql::new(url)?)),
            Some(SqlFamily::Sqlite) => Ok(GenericSqlConnection::Sqlite(Sqlite::new(url_str, db_name.unwrap_or("db"))?)),
            None => panic!("Unsupported database URL scheme: {}", url.scheme()),
        }
    }

    pub fn sql_family(&self) -> SqlFamily {
        match self {
            GenericSqlConnection::Postgresql(_) => SqlFamily::Postgres,
            GenericSqlConnection::Mysql(_) => SqlFamily::Mysql,
            GenericSqlConnection::Sqlite(_) => SqlFamily::Sqlite,
        }
    }

    pub fn connection_info(&self) -> ConnectionInfo {
        match self {
            GenericSqlConnection::Postgresql(pg) => ConnectionInfo::Postgres(pg.url()),
            GenericSqlConnection::Mysql(mysql) => ConnectionInfo::Mysql(mysql.url()),
            GenericSqlConnection::Sqlite(sqlite) => {
                ConnectionInfo::Sqlite { file_path: sqlite.file_path().to_owned() }
            },
        }
    }

    fn as_sql_connection(&self) -> &dyn SqlConnection {
        match self {
            GenericSqlConnection::Postgresql(pg) => pg,
            GenericSqlConnection::Mysql(mysql) => mysql,
            GenericSqlConnection::Sqlite(sqlite) => sqlite,
        }
    }

    fn as_sync_sql_connection(&self) -> &dyn SyncSqlConnection {
        match self {
            GenericSqlConnection::Postgresql(pg) => pg,
            GenericSqlConnection::Mysql(mysql) => mysql,
            GenericSqlConnection::Sqlite(sqlite) => sqlite,
        }
    }
}

#[async_trait::async_trait]
impl SqlConnection for GenericSqlConnection {
    async fn execute<'a>(&self, q: Query<'a>) -> Result<Option<Id>, QuaintError> {
        self.as_sql_connection().execute(q).await
    }

    async fn query<'a>(&self, q: Query<'a>) -> Result<ResultSet, QuaintError> {
        self.as_sql_connection().query(q).await
    }

    async fn query_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<ResultSet, QuaintError> {
        self.as_sql_connection().query_raw(sql, params).await
    }

    async fn execute_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QuaintError> {
        self.as_sql_connection().execute_raw(sql, params).await
    }
}

impl SyncSqlConnection for GenericSqlConnection {
    fn execute(&self, q: Query<'_>) -> Result<Option<Id>, QuaintError> {
        self.as_sync_sql_connection().execute(q)
    }

    fn query(&self, q: Query<'_>) -> Result<ResultSet, QuaintError> {
        self.as_sync_sql_connection().query(q)
    }

    fn query_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<ResultSet, QuaintError> {
        self.as_sync_sql_connection().query_raw(sql, params)
    }

    fn execute_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<u64, QuaintError> {
        self.as_sync_sql_connection().execute_raw(sql, params)
    }
}
