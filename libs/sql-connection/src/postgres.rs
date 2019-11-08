use crate::{pooling::*, traits::*};
use quaint::{
    ast::*,
    connector::{self, ResultSet},
    error::Error as QueryError,
    pool::{PostgresManager},
};
use std::convert::{TryInto};
use tokio::runtime::Runtime;
use url::Url;

/// A connection, or pool of connections, to a Postgres database. It exposes both sync and async
/// query interfaces.
pub struct Postgresql {
    // TODO: remove this when we delete the sync interface
    runtime: Runtime,
    conn: ConnectionPool<connector::PostgreSql, PostgresManager>,
}

impl Postgresql {
    /// Create a new connection pool.
    pub fn new_pooled(url: Url) -> Result<Self, QueryError> {
        let pool = quaint::pool::postgres(url)?;
        let handle = ConnectionPool::Pool(pool);

        Ok(Postgresql {
            conn: handle,
            runtime: super::default_runtime(),
        })
    }

    /// Create a new single connection.
    pub fn new_unpooled(url: Url) -> Result<Self, QueryError> {
        let runtime = super::default_runtime();
        let conn = runtime.block_on(connector::PostgreSql::from_params(url.try_into()?))?;
        let handle = ConnectionPool::Single(conn);

        Ok(Postgresql { conn: handle, runtime })
    }

    async fn get_connection<'a>(
        &'a self,
    ) -> Result<ConnectionHandle<'a, connector::PostgreSql, PostgresManager>, QueryError> {
        Ok(self.conn.get_connection().await?)
    }
}

#[async_trait::async_trait]
impl SqlConnection for Postgresql {
    async fn execute<'a>(&self, q: Query<'a>) -> Result<Option<Id>, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().execute(q).await
    }

    async fn query<'a>(&self, q: Query<'a>) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().query(q).await
    }

    async fn query_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().query_raw(sql, params).await
    }

    async fn execute_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().execute_raw(sql, params).await
    }
}

impl SyncSqlConnection for Postgresql {
    fn execute(&self, q: Query<'_>) -> Result<Option<Id>, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().execute(q))
    }

    fn query(&self, q: Query<'_>) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().query(q))
    }

    fn query_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().query_raw(sql, params))
    }

    fn execute_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<u64, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().execute_raw(sql, params))
    }
}
