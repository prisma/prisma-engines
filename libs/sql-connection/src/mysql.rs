use crate::{traits::{SqlConnection, SyncSqlConnection}};
use quaint::{
    ast::*,
    connector::{ResultSet, MysqlUrl, Queryable},
    error::Error as QueryError,
    pool::{Pool, MysqlManager},
};
use tokio::runtime::Runtime;
use url::Url;

/// A connection, or pool of connections, to a MySQL database. It exposes both sync and async
/// query interfaces.
pub struct Mysql {
    pool: Pool<MysqlManager>,
    url: MysqlUrl,
    // TODO: remove this when we delete the sync interface
    runtime: Runtime,
}

impl Mysql {
    /// Create a new connection pool.
    pub fn new(url: Url) -> Result<Self, QueryError> {
        let pool = quaint::pool::mysql(url.clone())?;

        Ok(Mysql {
            pool,
            url: MysqlUrl::new(url)?,
            runtime: super::default_runtime(),
        })
    }

    pub(crate) fn url(&self) -> MysqlUrl {
        self.url.clone()
    }
}

#[async_trait::async_trait]
impl SqlConnection for Mysql {
    async fn execute<'a>(&self, q: Query<'a>) -> Result<Option<Id>, QueryError> {
        let conn = self.pool.check_out().await?;
        conn.execute(q).await
    }

    async fn query<'a>(&self, q: Query<'a>) -> Result<ResultSet, QueryError> {
        let conn = self.pool.check_out().await?;
        conn.query(q).await
    }

    async fn query_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<ResultSet, QueryError> {
        let conn = self.pool.check_out().await?;
        conn.query_raw(sql, params).await
    }

    async fn execute_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QueryError> {
        let conn = self.pool.check_out().await?;
        conn.execute_raw(sql, params).await
    }
}

impl SyncSqlConnection for Mysql {
    fn execute(&self, q: Query<'_>) -> Result<Option<Id>, QueryError> {
        let conn = self.runtime.block_on(self.pool.check_out())?;
        self.runtime.block_on(conn.execute(q))
    }

    fn query(&self, q: Query<'_>) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.pool.check_out())?;
        self.runtime.block_on(conn.query(q))
    }

    fn query_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.pool.check_out())?;
        self.runtime.block_on(conn.query_raw(sql, params))
    }

    fn execute_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<u64, QueryError> {
        let conn = self.runtime.block_on(self.pool.check_out())?;
        self.runtime.block_on(conn.execute_raw(sql, params))
    }
}
