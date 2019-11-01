#![deny(missing_docs, rust_2018_idioms)]

//! Shared SQL connection handling logic for the migration engine and the introspection engine.

use prisma_query::{
    ast::*,
    connector::{self, Queryable, ResultSet, SqliteParams},
    error::Error as QueryError,
    pool::{CheckOut, Manage, MysqlManager, Pool, PostgresManager, SqliteManager},
};
use std::convert::{TryFrom, TryInto};
use tokio::runtime::Runtime;
use url::Url;

/// A generic synchronous SQL connection interface.
pub trait SyncSqlConnection {
    /// See
    /// https://prisma.github.io/prisma-query/prisma_query/connector/trait.Queryable.html#tymethod.execute
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    fn execute(&self, q: Query<'_>) -> Result<Option<Id>, QueryError>;

    /// See
    /// https://prisma.github.io/prisma-query/prisma_query/connector/trait.Queryable.html#tymethod.query
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    fn query(&self, q: Query<'_>) -> Result<ResultSet, QueryError>;

    /// See
    /// https://prisma.github.io/prisma-query/prisma_query/connector/trait.Queryable.html#tymethod.query_raw
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    fn query_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<ResultSet, QueryError>;

    /// See
    /// https://prisma.github.io/prisma-query/prisma_query/connector/trait.Queryable.html#tymethod.execute_raw
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    fn execute_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<u64, QueryError>;
}

/// A generic asynchronous SQL connection interface.
#[async_trait::async_trait]
pub trait SqlConnection: Send + Sync + 'static {
    /// See
    /// https://prisma.github.io/prisma-query/prisma_query/connector/trait.Queryable.html#tymethod.execute
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    async fn execute<'a>(&self, q: Query<'a>) -> Result<Option<Id>, QueryError>;

    /// See
    /// https://prisma.github.io/prisma-query/prisma_query/connector/trait.Queryable.html#tymethod.query
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    async fn query<'a>(&self, q: Query<'a>) -> Result<ResultSet, QueryError>;

    /// See
    /// https://prisma.github.io/prisma-query/prisma_query/connector/trait.Queryable.html#tymethod.query_raw
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    async fn query_raw<'a>(
        &self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<ResultSet, QueryError>;

    /// See
    /// https://prisma.github.io/prisma-query/prisma_query/connector/trait.Queryable.html#tymethod.execute_raw
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    async fn execute_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QueryError>;
}

type SqlitePool = Pool<SqliteManager>;

/// A pooled of connections to an SQLite database. It exposes both sync and async query
/// interfaces.
pub struct Sqlite {
    pool: SqlitePool,
    file_path: String,
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
        let file_path = params.file_path.to_str().unwrap().to_string();

        let pool = prisma_query::pool::sqlite(url, db_name)?;

        Ok(Self {
            pool,
            file_path,
            runtime: default_runtime(),
        })
    }

    /// The filesystem path of connection's database.
    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
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

    async fn query_raw<'a>(
        &self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<ResultSet, QueryError> {
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

/// A handle to a database connection pool that works generically over a pool of a single
/// connection or a prisma query connection pool.
enum ConnectionPool<C, P>
where
    C: Queryable + Send + Sync,
    P: Manage<Resource = C>,
{
    Single(C),
    Pool(Pool<P>),
}

impl<C, P> ConnectionPool<C, P>
where
    C: Queryable + Send + 'static,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    async fn get_connection<'a>(&'a self) -> Result<ConnectionHandle<'a, C, P>, QueryError> {
        match &self {
            ConnectionPool::Single(conn) => {
                Ok(ConnectionHandle::Single(conn))
            }
            ConnectionPool::Pool(pool) => {
                let checkout: CheckOut<P> = pool.check_out().await?;
                Ok(ConnectionHandle::PoolCheckout(checkout))
            }
        }
    }
}

/// A handle to a single connection from a [`ConnectionPool`](/enum.ConnectionPool.html)).
enum ConnectionHandle<'a, C, P>
where
    C: Queryable + Send + Sync + 'static,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    Single(&'a C),
    PoolCheckout(CheckOut<P>),
}

impl<'a, C, P> ConnectionHandle<'a, C, P>
where
    C: Queryable + Send,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    fn as_queryable(&self) -> &dyn Queryable {
        match self {
            ConnectionHandle::Single(guard) => guard,
            ConnectionHandle::PoolCheckout(co) => co,
        }
    }
}

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
        let pool = prisma_query::pool::postgres(url)?;
        let handle = ConnectionPool::Pool(pool);

        Ok(Postgresql {
            conn: handle,
            runtime: default_runtime(),
        })
    }

    /// Create a new single connection.
    pub fn new_unpooled(url: Url) -> Result<Self, QueryError> {
        let runtime = default_runtime();
        let conn = runtime.block_on(connector::PostgreSql::from_params(url.try_into()?))?;
        let handle = ConnectionPool::Single(conn);

        Ok(Postgresql { conn: handle, runtime })
    }

    async fn get_connection<'a>(&'a self) -> Result<ConnectionHandle<'a, connector::PostgreSql, PostgresManager>, QueryError> {
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

    async fn query_raw<'a>(
        &self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<ResultSet, QueryError> {
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

/// A connection, or pool of connections, to a MySQL database. It exposes both sync and async
/// query interfaces.
pub struct Mysql {
    conn: ConnectionPool<connector::Mysql, MysqlManager>,
    // TODO: remove this when we delete the sync interface
    runtime: Runtime,
}

impl Mysql {
    /// Create a new single connection.
    pub fn new_unpooled(url: Url) -> Result<Self, QueryError> {
        let conn = connector::Mysql::from_params(url.try_into()?)?;
        let handle = ConnectionPool::Single(conn);

        Ok(Mysql {
            conn: handle,
            runtime: default_runtime(),
        })
    }

    /// Create a new connection pool.
    pub fn new_pooled(url: Url) -> Result<Self, QueryError> {
        let pool = prisma_query::pool::mysql(url)?;
        let handle = ConnectionPool::Pool(pool);

        Ok(Mysql {
            conn: handle,
            runtime: default_runtime(),
        })
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

    async fn query_raw<'a>(
        &self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> Result<ResultSet, QueryError> {
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

fn default_runtime() -> Runtime {
    Runtime::new().expect("failed to start tokio runtime")
}
