#[cfg(feature = "mysql")]
use crate::connector::MysqlUrl;
#[cfg(feature = "postgresql")]
use crate::connector::PostgresUrl;

use crate::{
    ast,
    connector::{self, Queryable, TransactionCapable, DBIO},
    error::Error,
};
use futures::{future::FutureExt};
use mobc::{runtime::DefaultExecutor, AnyFuture, ConnectionManager, PooledConnection as MobcPooled};

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

    fn query_raw<'a>(&'a self, sql: &'a str, params: &'a [ast::ParameterizedValue]) -> DBIO<'a, connector::ResultSet> {
        self.inner.query_raw(sql, params)
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

impl ConnectionManager for QuaintManager {
    type Connection = Box<dyn Queryable + Send + Sync>;
    type Executor = DefaultExecutor;
    type Error = Error;

    fn get_executor(&self) -> Self::Executor {
        DefaultExecutor::current()
    }

    fn connect(&self) -> AnyFuture<Self::Connection, Self::Error> {
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite { file_path, db_name } => {
                use crate::connector::Sqlite;

                match Sqlite::new(&file_path) {
                    Ok(mut conn) => match conn.attach_database(db_name) {
                        Ok(_) => futures::future::ok(Box::new(conn) as Self::Connection).boxed(),
                        Err(e) => futures::future::err(e).boxed(),
                    },
                    Err(e) => futures::future::err(e).boxed(),
                }
            }

            #[cfg(feature = "mysql")]
            Self::Mysql(url) => {
                use crate::connector::Mysql;

                match Mysql::new(url.clone()) {
                    Ok(mysql) => futures::future::ok(Box::new(mysql) as Self::Connection).boxed(),
                    Err(e) => futures::future::err(e).boxed(),
                }
            }

            #[cfg(feature = "postgresql")]
            Self::Postgres(url) => {
                use crate::connector::PostgreSql;

                let url: PostgresUrl = url.clone();

                let fut = async move {
                    let conn = PostgreSql::new(url).await?;
                    Ok(Box::new(conn) as Self::Connection)
                };

                fut.boxed()
            }
        }
    }

    fn is_valid(&self, conn: Self::Connection) -> AnyFuture<Self::Connection, Self::Error> {
        async move {
            conn.query_raw("SELECT 1", &[]).await?;
            Ok(conn)
        }
            .boxed()
    }

    fn has_broken(&self, _: &mut Option<Self::Connection>) -> bool {
        false
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
