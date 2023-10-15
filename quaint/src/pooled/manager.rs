use std::future::Future;

use async_trait::async_trait;
use mobc::{Connection as MobcPooled, Manager};
use std::borrow::Cow;
use tracing_futures::WithSubscriber;

#[cfg(feature = "mssql-native")]
use crate::connector::MssqlUrl;
#[cfg(feature = "mysql-native")]
use crate::connector::MysqlUrl;
#[cfg(feature = "postgresql-native")]
use crate::connector::{MakeTlsConnectorManager, PostgresNativeUrl};
use crate::{
    ast,
    connector::{self, IsolationLevel, Queryable, Transaction, TransactionCapable},
    error::Error,
};

/// A connection from the pool. Implements
/// [Queryable](connector/trait.Queryable.html).
pub struct PooledConnection {
    pub(crate) inner: MobcPooled<QuaintManager>,
}

#[async_trait]
impl TransactionCapable for PooledConnection {
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> crate::Result<Box<dyn Transaction + 'a>> {
        self.inner.start_transaction(isolation).await
    }
}

#[async_trait]
impl Queryable for PooledConnection {
    async fn query(&self, q: ast::Query<'_>) -> crate::Result<connector::ResultSet> {
        self.inner.query(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[ast::Value<'_>]) -> crate::Result<connector::ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[ast::Value<'_>]) -> crate::Result<connector::ResultSet> {
        self.inner.query_raw_typed(sql, params).await
    }

    async fn describe_query(&self, sql: &str) -> crate::Result<connector::DescribedQuery> {
        self.inner.describe_query(sql).await
    }

    async fn execute(&self, q: ast::Query<'_>) -> crate::Result<u64> {
        self.inner.execute(q).await
    }

    async fn execute_raw(&self, sql: &str, params: &[ast::Value<'_>]) -> crate::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[ast::Value<'_>]) -> crate::Result<u64> {
        self.inner.execute_raw_typed(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        self.inner.raw_cmd(cmd).await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        self.inner.version().await
    }

    fn is_healthy(&self) -> bool {
        self.inner.is_healthy()
    }

    async fn server_reset_query(&self, tx: &dyn Transaction) -> crate::Result<()> {
        self.inner.server_reset_query(tx).await
    }

    fn begin_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.begin_statement(depth)
    }

    fn commit_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.commit_statement(depth)
    }

    fn rollback_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.rollback_statement(depth)
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        self.inner.set_tx_isolation_level(isolation_level).await
    }

    fn requires_isolation_first(&self) -> bool {
        self.inner.requires_isolation_first()
    }
}

#[doc(hidden)]
pub enum QuaintManager {
    #[cfg(feature = "mysql")]
    Mysql { url: MysqlUrl },

    #[cfg(feature = "postgresql")]
    Postgres {
        url: PostgresNativeUrl,
        tls_manager: Box<MakeTlsConnectorManager>,
        is_tracing_enabled: bool,
    },

    #[cfg(feature = "sqlite")]
    Sqlite { url: String, db_name: String },

    #[cfg(feature = "mssql")]
    Mssql { url: MssqlUrl },
}

#[async_trait]
impl Manager for QuaintManager {
    type Connection = Box<dyn TransactionCapable>;
    type Error = Error;

    async fn connect(&self) -> crate::Result<Self::Connection> {
        let conn = match self {
            #[cfg(feature = "sqlite-native")]
            QuaintManager::Sqlite { url, .. } => {
                use crate::connector::Sqlite;

                let conn = Sqlite::new(url)?;

                Ok(Box::new(conn) as Self::Connection)
            }

            #[cfg(feature = "mysql-native")]
            QuaintManager::Mysql { url } => {
                use crate::connector::Mysql;
                Ok(Box::new(Mysql::new(url.clone()).await?) as Self::Connection)
            }

            #[cfg(feature = "postgresql-native")]
            QuaintManager::Postgres {
                url,
                tls_manager,
                is_tracing_enabled: false,
            } => {
                use crate::connector::{PostgreSqlWithDefaultCache, PostgreSqlWithNoCache};
                Ok(if url.pg_bouncer() {
                    Box::new(PostgreSqlWithNoCache::new(url.clone(), tls_manager).await?) as Self::Connection
                } else {
                    Box::new(PostgreSqlWithDefaultCache::new(url.clone(), tls_manager).await?) as Self::Connection
                })
            }

            #[cfg(feature = "postgresql-native")]
            QuaintManager::Postgres {
                url,
                tls_manager,
                is_tracing_enabled: true,
            } => {
                use crate::connector::{PostgreSqlWithNoCache, PostgreSqlWithTracingCache};
                Ok(if url.pg_bouncer() {
                    Box::new(PostgreSqlWithNoCache::new(url.clone(), tls_manager).await?) as Self::Connection
                } else {
                    Box::new(PostgreSqlWithTracingCache::new(url.clone(), tls_manager).await?) as Self::Connection
                })
            }

            #[cfg(feature = "mssql-native")]
            QuaintManager::Mssql { url } => {
                use crate::connector::Mssql;
                Ok(Box::new(Mssql::new(url.clone()).await?) as Self::Connection)
            }
        };

        conn.iter()
            .for_each(|_| tracing::debug!("Acquired database connection."));

        conn
    }

    async fn check(&self, conn: Self::Connection) -> crate::Result<Self::Connection> {
        conn.raw_cmd("SELECT 1").await?;
        Ok(conn)
    }

    fn validate(&self, conn: &mut Self::Connection) -> bool {
        let single_use_connection = match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres { url, .. } => url.single_use_connections(),
            _ => false,
        };
        !single_use_connection && conn.is_healthy()
    }

    fn spawn_task<T>(&self, task: T)
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        tokio::spawn(task.with_current_subscriber());
    }
}

#[cfg(test)]
mod tests {
    use crate::pooled::Quaint;

    #[tokio::test]
    #[cfg(feature = "mysql-native")]
    async fn mysql_default_connection_limit() {
        let conn_string = std::env::var("TEST_MYSQL").expect("TEST_MYSQL connection string not set.");

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "mysql-native")]
    async fn mysql_custom_connection_limit() {
        let conn_string = format!(
            "{}?connection_limit=10",
            std::env::var("TEST_MYSQL").expect("TEST_MYSQL connection string not set.")
        );

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(10, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "postgresql-native")]
    async fn psql_default_connection_limit() {
        let conn_string = std::env::var("TEST_PSQL").expect("TEST_PSQL connection string not set.");

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "postgresql-native")]
    async fn psql_custom_connection_limit() {
        let conn_string = format!(
            "{}?connection_limit=10",
            std::env::var("TEST_PSQL").expect("TEST_PSQL connection string not set.")
        );

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(10, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "mssql-native")]
    async fn mssql_default_connection_limit() {
        let conn_string = std::env::var("TEST_MSSQL").expect("TEST_MSSQL connection string not set.");

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "mssql-native")]
    async fn mssql_custom_connection_limit() {
        let conn_string = format!(
            "{};connectionLimit=10",
            std::env::var("TEST_MSSQL").expect("TEST_MSSQL connection string not set.")
        );

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(10, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "sqlite-native")]
    async fn test_default_connection_limit() {
        let conn_string = "file:db/test.db".to_string();
        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "sqlite-native")]
    async fn test_custom_connection_limit() {
        let conn_string = "file:db/test.db?connection_limit=10".to_string();
        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(10, pool.capacity().await as usize);
    }
}
