#[cfg(feature = "mssql-native")]
use crate::connector::MssqlUrl;
#[cfg(feature = "mysql-native")]
use crate::connector::MysqlUrl;
#[cfg(feature = "postgresql-native")]
use crate::connector::PostgresUrl;
use crate::{
    ast,
    connector::{self, impl_default_TransactionCapable, IsolationLevel, Queryable, Transaction, TransactionCapable},
    error::Error,
};
use async_trait::async_trait;
use futures::lock::Mutex;
use mobc::{Connection as MobcPooled, Manager};
use std::sync::Arc;

/// A connection from the pool. Implements
/// [Queryable](connector/trait.Queryable.html).
pub struct PooledConnection {
    pub(crate) inner: MobcPooled<QuaintManager>,
    pub transaction_depth: Arc<Mutex<i32>>,
}

impl_default_TransactionCapable!(PooledConnection);

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

    async fn begin_statement(&self, depth: i32) -> String {
        self.inner.begin_statement(depth).await
    }

    async fn commit_statement(&self, depth: i32) -> String {
        self.inner.commit_statement(depth).await
    }

    async fn rollback_statement(&self, depth: i32) -> String {
        self.inner.rollback_statement(depth).await
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
    Postgres { url: PostgresUrl },

    #[cfg(feature = "sqlite")]
    Sqlite { url: String, db_name: String },

    #[cfg(feature = "mssql")]
    Mssql { url: MssqlUrl },
}

#[async_trait]
impl Manager for QuaintManager {
    type Connection = Box<dyn Queryable>;
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
            QuaintManager::Postgres { url } => {
                use crate::connector::PostgreSql;
                Ok(Box::new(PostgreSql::new(url.clone()).await?) as Self::Connection)
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
        conn.is_healthy()
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
