use crate::{traits::*};
use quaint::{
    ast::*,
    connector::{ResultSet, PostgresUrl, Queryable},
    error::Error as QueryError,
    pool::{Pool, PostgresManager},
};
use tokio::runtime::Runtime;
use url::Url;

/// A connection, or pool of connections, to a Postgres database. It exposes both sync and async
/// query interfaces.
pub struct Postgresql {
    // TODO: remove this when we delete the sync interface
    runtime: Runtime,
    pool: Pool<PostgresManager>,
    url: PostgresUrl,
}

impl Postgresql {
    /// Create a new connection pool.
    pub fn new(url: Url) -> Result<Self, QueryError> {
        let pool = quaint::pool::postgres(url.clone())?;

        Ok(Postgresql {
            pool,
            url: PostgresUrl(url),
            runtime: super::default_runtime(),
        })
    }

    pub(crate) fn url(&self) -> PostgresUrl {
        self.url.clone()
    }
}


#[async_trait::async_trait]
impl SqlConnection for Postgresql {
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

impl SyncSqlConnection for Postgresql {
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
