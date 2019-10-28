use async_std::sync::{Mutex, MutexGuard};
use prisma_query::{
    ast::*,
    connector::{self, Queryable, ResultSet, SqliteParams},
    error::Error as QueryError,
    pool::{MysqlManager, Pool, PostgresManager, SqliteManager},
};
use std::convert::{TryFrom, TryInto};
use tokio::runtime::Runtime;
use tokio_resource_pool::{CheckOut, Manage};
use url::Url;

pub trait SyncSqlConnection {
    fn execute(&self, db: &str, q: Query) -> Result<Option<Id>, QueryError>;
    fn query(&self, db: &str, q: Query) -> Result<ResultSet, QueryError>;
    fn query_raw(&self, db: &str, sql: &str, params: &[ParameterizedValue]) -> Result<ResultSet, QueryError>;
    fn execute_raw(&self, db: &str, sql: &str, params: &[ParameterizedValue]) -> Result<u64, QueryError>;
}

#[async_trait::async_trait]
pub trait SqlConnection: Send + Sync + 'static {
    async fn execute<'a>(&self, db: &str, q: Query<'a>) -> Result<Option<Id>, QueryError>;
    async fn query<'a>(&self, db: &str, q: Query<'a>) -> Result<ResultSet, QueryError>;
    async fn query_raw<'a>(
        &self,
        db: &str,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<ResultSet, QueryError>;
    async fn execute_raw<'a>(&self, db: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QueryError>;
}

type SqlitePool = Pool<SqliteManager>;

pub struct Sqlite {
    pool: SqlitePool,
    file_path: String,
    // TODO: remove this when we remove the sync API
    runtime: Runtime,
}

impl Sqlite {
    pub fn new(url: &str) -> Result<Self, QueryError> {
        let params = SqliteParams::try_from(url)?;
        let file_path = params.file_path.to_str().unwrap().to_string();

        let pool = prisma_query::pool::sqlite(url)?;

        Ok(Self {
            pool,
            file_path,
            runtime: default_runtime(),
        })
    }

    async fn get_connection(&self) -> Result<CheckOut<SqliteManager>, QueryError> {
        self.pool.check_out().await
    }

    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }
}

#[async_trait::async_trait]
impl SqlConnection for Sqlite {
    async fn execute<'a>(&self, _: &str, q: Query<'a>) -> Result<Option<Id>, QueryError> {
        let conn = self.get_connection().await?;
        conn.execute(q).await
    }

    async fn query<'a>(&self, _: &str, q: Query<'a>) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.query(q).await
    }

    async fn query_raw<'a>(
        &self,
        _: &str,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.query_raw(sql, params).await
    }

    async fn execute_raw<'a>(&self, _: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QueryError> {
        let conn = self.get_connection().await?;
        conn.execute_raw(sql, params).await
    }
}

impl SyncSqlConnection for Sqlite {
    fn execute(&self, _db: &str, q: Query) -> Result<Option<Id>, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.execute(q))
    }

    fn query(&self, _db: &str, q: Query) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.query(q))
    }

    fn query_raw(&self, _db: &str, sql: &str, params: &[ParameterizedValue]) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.query_raw(sql, params))
    }

    fn execute_raw(&self, _db: &str, sql: &str, params: &[ParameterizedValue]) -> Result<u64, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.execute_raw(sql, params))
    }
}

/// A handle to a database connection that works generically over a single connection behind a
/// mutex or a prisma query connection pool.
enum ConnectionHandle<C, P>
where
    C: Queryable + Send + Sync,
    P: Manage<Resource = C>,
{
    Single(Mutex<C>),
    Pool(Pool<P>),
}

impl<C, P> ConnectionHandle<C, P>
where
    C: Queryable + Send + 'static,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    async fn get_connection<'a>(&'a self) -> Result<CH<'a, C, P>, QueryError> {
        match &self {
            ConnectionHandle::Single(mutex) => {
                let guard = mutex.lock().await;
                Ok(CH::Single(guard))
            }
            ConnectionHandle::Pool(pool) => {
                let checkout: CheckOut<P> = pool.check_out().await?;
                Ok(CH::PoolCheckout(checkout))
            }
        }
    }
}

enum CH<'a, C, P>
where
    C: Queryable + Send + Sync + 'static,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    Single(MutexGuard<'a, C>),
    PoolCheckout(CheckOut<P>),
}

impl<'a, C, P> CH<'a, C, P>
where
    C: Queryable + Send,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    fn as_queryable(&self) -> &dyn Queryable {
        match self {
            CH::Single(guard) => guard,
            CH::PoolCheckout(co) => co,
        }
    }
}

pub struct Postgresql {
    // TODO: remove this when we delete the sync interface
    runtime: Runtime,
    conn: ConnectionHandle<connector::PostgreSql, PostgresManager>,
}

impl Postgresql {
    pub fn new_pooled(url: Url) -> Result<Self, QueryError> {
        let pool = prisma_query::pool::postgres(url)?;
        let handle = ConnectionHandle::Pool(pool);

        Ok(Postgresql {
            conn: handle,
            runtime: default_runtime(),
        })
    }

    pub fn new_unpooled(url: Url) -> Result<Self, QueryError> {
        let runtime = default_runtime();
        let conn = runtime.block_on(connector::PostgreSql::from_params(url.try_into()?))?;
        let handle = ConnectionHandle::Single(Mutex::new(conn));

        Ok(Postgresql { conn: handle, runtime })
    }

    async fn get_connection<'a>(&'a self) -> Result<CH<'a, connector::PostgreSql, PostgresManager>, QueryError> {
        Ok(self.conn.get_connection().await?)
    }
}

#[async_trait::async_trait]
impl SqlConnection for Postgresql {
    async fn execute<'a>(&self, _: &str, q: Query<'a>) -> Result<Option<Id>, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().execute(q).await
    }

    async fn query<'a>(&self, _: &str, q: Query<'a>) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().query(q).await
    }

    async fn query_raw<'a>(
        &self,
        _: &str,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().query_raw(sql, params).await
    }

    async fn execute_raw<'a>(
        &self,
        _: &str,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<u64, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().execute_raw(sql, params).await
    }
}

impl SyncSqlConnection for Postgresql {
    fn execute(&self, _db: &str, q: Query) -> Result<Option<Id>, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().execute(q))
    }

    fn query(&self, _db: &str, q: Query) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().query(q))
    }

    fn query_raw(&self, _db: &str, sql: &str, params: &[ParameterizedValue]) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().query_raw(sql, params))
    }

    fn execute_raw(&self, _db: &str, sql: &str, params: &[ParameterizedValue]) -> Result<u64, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().execute_raw(sql, params))
    }
}

pub struct Mysql {
    conn: ConnectionHandle<connector::Mysql, MysqlManager>,
    // TODO: remove this when we delete the sync interface
    runtime: Runtime,
}

impl Mysql {
    pub fn new_unpooled(url: Url) -> Result<Self, QueryError> {
        let conn = connector::Mysql::from_params(url.try_into()?)?;
        let handle = ConnectionHandle::Single(Mutex::new(conn));

        Ok(Mysql {
            conn: handle,
            runtime: default_runtime(),
        })
    }

    pub fn new_pooled(url: Url) -> Result<Self, QueryError> {
        let pool = prisma_query::pool::mysql(url)?;
        let handle = ConnectionHandle::Pool(pool);

        Ok(Mysql {
            conn: handle,
            runtime: default_runtime(),
        })
    }

    async fn get_connection<'a>(&'a self) -> Result<CH<'a, connector::Mysql, MysqlManager>, QueryError> {
        Ok(self.conn.get_connection().await?)
    }
}

#[async_trait::async_trait]
impl SqlConnection for Mysql {
    async fn execute<'a>(&self, _: &str, q: Query<'a>) -> Result<Option<Id>, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().execute(q).await
    }

    async fn query<'a>(&self, _: &str, q: Query<'a>) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().query(q).await
    }

    async fn query_raw<'a>(
        &self,
        _: &str,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<ResultSet, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().query_raw(sql, params).await
    }

    async fn execute_raw<'a>(&self, _: &str, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QueryError> {
        let conn = self.get_connection().await?;
        conn.as_queryable().execute_raw(sql, params).await
    }
}

impl SyncSqlConnection for Mysql {
    fn execute(&self, _db: &str, q: Query) -> Result<Option<Id>, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().execute(q))
    }

    fn query(&self, _db: &str, q: Query) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().query(q))
    }

    fn query_raw(&self, _db: &str, sql: &str, params: &[ParameterizedValue]) -> Result<ResultSet, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().query_raw(sql, params))
    }

    fn execute_raw(&self, _db: &str, sql: &str, params: &[ParameterizedValue]) -> Result<u64, QueryError> {
        let conn = self.runtime.block_on(self.get_connection())?;
        self.runtime.block_on(conn.as_queryable().execute_raw(sql, params))
    }
}

fn default_runtime() -> Runtime {
    Runtime::new().expect("failed to start tokio runtime")
}
