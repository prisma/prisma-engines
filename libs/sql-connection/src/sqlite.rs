use crate::traits::{SqlConnection, SyncSqlConnection};
use quaint::{
    ast::*,
    connector::{Queryable, ResultSet, SqliteParams},
    error::Error as QueryError,
    pool::{CheckOut, Pool, SqliteManager},
};
use std::convert::{TryFrom};
use tokio::runtime::Runtime;

type SqlitePool = Pool<SqliteManager>;

/// A pooled of connections to an SQLite database. It exposes both sync and async query
/// interfaces.
pub struct Sqlite {
    pool: SqlitePool,
    file_path: String,
    db_name: String,
    // TODO: remove this when we remove the sync API
    runtime: Runtime,
}

impl Sqlite {
    /// Create a connection pool to an SQLite database.
    ///
    /// - `url` is the url or file path for the database.
    /// - `db_name` is the name the database will be attached to for all the connections in the pool.
    pub fn new(url: &str, db_name: &str) -> Result<Self, QueryError> {
        let params = SqliteParams::try_from(url)?;
        let file_path = params.file_path;

        let pool = quaint::pool::sqlite(url, db_name)?;

        Ok(Self {
            pool,
            db_name: db_name.to_owned(),
            file_path: file_path.to_owned(),
            runtime: super::default_runtime(),
        })
    }

    /// The filesystem path of connection's database.
    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }

    /// The name the database is bound to (with `ATTACH DATABASE`).
    pub fn db_name(&self) -> &str {
        self.db_name.as_str()
    }

    async fn get_connection(&self) -> Result<CheckOut<SqliteManager>, QueryError> {
        Ok(self.pool.check_out().await?)
    }
}

#[async_trait::async_trait]
impl SqlConnection for Sqlite {
    async fn execute<'a>(&self, q: Query<'a>) -> Result<Option<Id>, QueryError> {
        let conn = self.get_connection().await?;
        conn.execute(q).await
    }

    async fn query<'a>(&self, q: Query<'a>) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.query(q).await
    }

    async fn query_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.query_raw(sql, params).await
    }

    async fn execute_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QueryError> {
        let conn = self.get_connection().await?;
        conn.execute_raw(sql, params).await
    }
}

impl SyncSqlConnection for Sqlite {
    fn execute(&self, q: Query<'_>) -> Result<Option<Id>, QueryError> {
        self.runtime.block_on(<Self as SqlConnection>::execute(self, q))
    }

    fn query(&self, q: Query<'_>) -> Result<ResultSet, QueryError> {
        self.runtime.block_on(<Self as SqlConnection>::query(self, q))
    }

    fn query_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<ResultSet, QueryError> {
        self.runtime
            .block_on(<Self as SqlConnection>::query_raw(self, sql, params))
    }

    fn execute_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<u64, QueryError> {
        self.runtime
            .block_on(<Self as SqlConnection>::execute_raw(self, sql, params))
    }
}
