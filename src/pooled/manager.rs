#[cfg(feature = "mssql")]
use crate::connector::MssqlUrl;
#[cfg(feature = "mysql")]
use crate::connector::MysqlUrl;
#[cfg(feature = "postgresql")]
use crate::connector::PostgresUrl;
use crate::{
    ast,
    connector::{self, Queryable, Transaction, TransactionCapable},
    error::Error,
};
use async_trait::async_trait;
use mobc::{Connection as MobcPooled, Manager};

/// A connection from the pool. Implements
/// [Queryable](connector/trait.Queryable.html).
pub struct PooledConnection {
    pub(crate) inner: MobcPooled<QuaintManager>,
}

impl TransactionCapable for PooledConnection {}

#[async_trait]
impl Queryable for PooledConnection {
    async fn query(&self, q: ast::Query<'_>) -> crate::Result<connector::ResultSet> {
        self.inner.query(q).await
    }

    async fn execute(&self, q: ast::Query<'_>) -> crate::Result<u64> {
        self.inner.execute(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[ast::Value<'_>]) -> crate::Result<connector::ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[ast::Value<'_>]) -> crate::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        self.inner.raw_cmd(cmd).await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        self.inner.version().await
    }

    async fn server_reset_query(&self, tx: &Transaction<'_>) -> crate::Result<()> {
        self.inner.server_reset_query(tx).await
    }

    fn begin_statement(&self) -> &'static str {
        self.inner.begin_statement()
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
            #[cfg(feature = "sqlite")]
            QuaintManager::Sqlite { url, .. } => {
                use crate::connector::Sqlite;

                let conn = Sqlite::new(&url)?;

                Ok(Box::new(conn) as Self::Connection)
            }

            #[cfg(feature = "mysql")]
            QuaintManager::Mysql { url } => {
                use crate::connector::Mysql;
                Ok(Box::new(Mysql::new(url.clone()).await?) as Self::Connection)
            }

            #[cfg(feature = "postgresql")]
            QuaintManager::Postgres { url } => {
                use crate::connector::PostgreSql;
                Ok(Box::new(PostgreSql::new(url.clone()).await?) as Self::Connection)
            }

            #[cfg(feature = "mssql")]
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
}

#[cfg(test)]
mod tests {
    use crate::pooled::Quaint;

    #[tokio::test]
    #[cfg(feature = "mysql")]
    async fn mysql_default_connection_limit() {
        let conn_string = std::env::var("TEST_MYSQL").expect("TEST_MYSQL connection string not set.");

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "mysql")]
    async fn mysql_custom_connection_limit() {
        let conn_string = format!(
            "{}?connection_limit=10",
            std::env::var("TEST_MYSQL").expect("TEST_MYSQL connection string not set.")
        );

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(10, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "postgresql")]
    async fn psql_default_connection_limit() {
        let conn_string = std::env::var("TEST_PSQL").expect("TEST_PSQL connection string not set.");

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "postgresql")]
    async fn psql_custom_connection_limit() {
        let conn_string = format!(
            "{}?connection_limit=10",
            std::env::var("TEST_PSQL").expect("TEST_PSQL connection string not set.")
        );

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(10, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "mssql")]
    async fn mssql_default_connection_limit() {
        let conn_string = std::env::var("TEST_MSSQL").expect("TEST_MSSQL connection string not set.");

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "mssql")]
    async fn mssql_custom_connection_limit() {
        let conn_string = format!(
            "{};connectionLimit=10",
            std::env::var("TEST_MSSQL").expect("TEST_MSSQL connection string not set.")
        );

        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(10, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "sqlite")]
    async fn test_default_connection_limit() {
        let conn_string = format!("file:db/test.db",);
        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "sqlite")]
    async fn test_custom_connection_limit() {
        let conn_string = format!("file:db/test.db?connection_limit=10",);
        let pool = Quaint::builder(&conn_string).unwrap().build();

        assert_eq!(10, pool.capacity().await as usize);
    }
}
