use async_trait::async_trait;

#[cfg(feature = "mysql")]
use crate::connector::MysqlUrl;
#[cfg(feature = "postgresql")]
use crate::connector::PostgresUrl;

use crate::{
    ast,
    connector::{self, Queryable, TransactionCapable, DBIO},
    error::Error,
};
use mobc::{Connection as MobcPooled, Manager};

/// A connection from the pool. Implements
/// [Queryable](connector/trait.Queryable.html).
pub struct PooledConnection {
    pub(crate) inner: MobcPooled<QuaintManager>,
}

impl TransactionCapable for PooledConnection {}

impl Queryable for PooledConnection {
    fn query<'a>(&'a self, q: ast::Query<'a>) -> DBIO<'a, connector::ResultSet> {
        self.inner.query(q)
    }

    fn execute<'a>(&'a self, q: ast::Query<'a>) -> DBIO<'a, u64> {
        self.inner.execute(q)
    }

    fn query_raw<'a>(&'a self, sql: &'a str, params: &'a [ast::ParameterizedValue]) -> DBIO<'a, connector::ResultSet> {
        self.inner.query_raw(sql, params)
    }

    fn execute_raw<'a>(&'a self, sql: &'a str, params: &'a [ast::ParameterizedValue]) -> DBIO<'a, u64> {
        self.inner.execute_raw(sql, params)
    }

    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()> {
        self.inner.raw_cmd(cmd)
    }
}

#[doc(hidden)]
pub enum QuaintManager {
    #[cfg(feature = "mysql")]
    Mysql(MysqlUrl),

    #[cfg(feature = "postgresql")]
    Postgres(PostgresUrl),

    #[cfg(feature = "sqlite")]
    Sqlite { file_path: String, db_name: String },
}

#[async_trait]
impl Manager for QuaintManager {
    type Connection = Box<dyn Queryable + Send + Sync>;
    type Error = Error;

    async fn connect(&self) -> crate::Result<Self::Connection> {
        match self {
            #[cfg(feature = "sqlite")]
            QuaintManager::Sqlite { file_path, db_name } => {
                use crate::connector::Sqlite;

                let mut conn = Sqlite::new(&file_path)?;
                conn.attach_database(db_name)?;

                Ok(Box::new(conn) as Self::Connection)
            }

            #[cfg(feature = "mysql")]
            QuaintManager::Mysql(url) => {
                use crate::connector::Mysql;
                Ok(Box::new(Mysql::new(url.clone())?) as Self::Connection)
            }

            #[cfg(feature = "postgresql")]
            QuaintManager::Postgres(url) => {
                use crate::connector::PostgreSql;
                Ok(Box::new(PostgreSql::new(url.clone()).await?) as Self::Connection)
            }
        }
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

        let pool = Quaint::new(&conn_string).await.unwrap();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "mysql")]
    async fn mysql_custom_connection_limit() {
        let conn_string = format!(
            "{}?connection_limit=10",
            std::env::var("TEST_MYSQL").expect("TEST_MYSQL connection string not set.")
        );

        let pool = Quaint::new(&conn_string).await.unwrap();

        assert_eq!(10, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "postgresql")]
    async fn psql_default_connection_limit() {
        let conn_string = std::env::var("TEST_PSQL").expect("TEST_PSQL connection string not set.");

        let pool = Quaint::new(&conn_string).await.unwrap();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "postgresql")]
    async fn psql_custom_connection_limit() {
        let conn_string = format!(
            "{}?connection_limit=10",
            std::env::var("TEST_PSQL").expect("TEST_PSQL connection string not set.")
        );

        let pool = Quaint::new(&conn_string).await.unwrap();

        assert_eq!(10, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "sqlite")]
    async fn test_default_connection_limit() {
        let conn_string = format!("file:db/test.db",);
        let pool = Quaint::new(&conn_string).await.unwrap();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity().await as usize);
    }

    #[tokio::test]
    #[cfg(feature = "sqlite")]
    async fn test_custom_connection_limit() {
        let conn_string = format!("file:db/test.db?connection_limit=10",);
        let pool = Quaint::new(&conn_string).await.unwrap();

        assert_eq!(10, pool.capacity().await as usize);
    }
}
