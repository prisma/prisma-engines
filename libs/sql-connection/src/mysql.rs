use crate::{pooling::*, traits::{SqlConnection, SyncSqlConnection}};
use quaint::{
    ast::*,
    connector::{self, ResultSet, MysqlUrl},
    error::Error as QueryError,
    pool::{MysqlManager},
};
use std::convert::{TryInto};
use tokio::runtime::Runtime;
use url::Url;

/// A connection, or pool of connections, to a MySQL database. It exposes both sync and async
/// query interfaces.
pub struct Mysql {
    conn: ConnectionPool<connector::Mysql, MysqlManager>,
    url: MysqlUrl,
    // TODO: remove this when we delete the sync interface
    runtime: Runtime,
}

impl Mysql {
    /// Create a new single connection.
    pub fn new_unpooled(url: Url) -> Result<Self, QueryError> {
        let conn = connector::Mysql::from_params(url.clone().try_into()?)?;
        let handle = ConnectionPool::Single(conn);

        Ok(Mysql {
            conn: handle,
            url: MysqlUrl(url),
            runtime: super::default_runtime(),
        })
    }

    /// Create a new connection pool.
    pub fn new_pooled(url: Url) -> Result<Self, QueryError> {
        let pool = quaint::pool::mysql(url.clone())?;
        let handle = ConnectionPool::Pool(pool);

        Ok(Mysql {
            conn: handle,
            url: MysqlUrl(url),
            runtime: super::default_runtime(),
        })
    }

    pub(crate) fn url(&self) -> MysqlUrl {
        self.url.clone()
    }

    async fn get_connection<'a>(&'a self) -> Result<ConnectionHandle<'a, connector::Mysql, MysqlManager>, QueryError> {
        Ok(self.conn.get_connection().await?)
    }
}

#[async_trait::async_trait]
impl SqlConnection for Mysql {
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

impl SyncSqlConnection for Mysql {
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

