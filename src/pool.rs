mod connection_info;

pub use connection_info::*;

use crate::{
    ast,
    connector::{self, Queryable, TransactionCapable, DBIO},
    error::Error,
};
use futures::future;
use tokio_resource_pool::{CheckOut, Manage, RealDependencies, Status};

/// A connection from the pool. Implements
/// [Queryable](connector/trait.Queryable.html).
pub struct PooledConnection {
    pub(crate) inner: CheckOut<QuaintManager>,
}

impl TransactionCapable for PooledConnection {}

impl Queryable for PooledConnection {
    fn execute<'a>(&'a self, q: ast::Query<'a>) -> DBIO<'a, Option<ast::Id>> {
        self.inner.execute(q)
    }

    fn query<'a>(&'a self, q: ast::Query<'a>) -> DBIO<'a, connector::ResultSet> {
        self.inner.query(q)
    }

    fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ast::ParameterizedValue],
    ) -> DBIO<'a, connector::ResultSet> {
        self.inner.query_raw(sql, params)
    }

    fn execute_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ast::ParameterizedValue],
    ) -> DBIO<'a, u64> {
        self.inner.execute_raw(sql, params)
    }

    fn turn_off_fk_constraints(&self) -> DBIO<()> {
        self.inner.turn_off_fk_constraints()
    }

    fn turn_on_fk_constraints(&self) -> DBIO<()> {
        self.inner.turn_on_fk_constraints()
    }

    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()> {
        self.inner.raw_cmd(cmd)
    }
}

#[doc(hidden)]
pub enum QuaintManager {
    #[cfg(feature = "mysql")]
    Mysql(mysql_async::OptsBuilder),
    #[cfg(feature = "postgresql")]
    Postgres {
        config: tokio_postgres::Config,
        schema: String,
        ssl_params: crate::connector::postgres::SslParams,
    },
    #[cfg(feature = "sqlite")]
    Sqlite {
        file_path: String,
        db_name: String,
    },
}

impl Manage for QuaintManager {
    type Resource = Box<dyn Queryable + Send + Sync>;
    type Dependencies = RealDependencies;
    type CheckOut = CheckOut<Self>;
    type Error = Error;
    type CreateFuture = DBIO<'static, Self::Resource>;
    type RecycleFuture = DBIO<'static, Option<Self::Resource>>;

    fn create(&self) -> Self::CreateFuture {
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite { file_path, db_name } => {
                use crate::connector::Sqlite;

                match Sqlite::new(&file_path) {
                    Ok(mut conn) => {
                         match conn.attach_database(db_name) {
                            Ok(_) => DBIO::new(future::ok(Box::new(conn) as Self::Resource)),
                            Err(e) => DBIO::new(future::err(e)),
                        }
                    },
                    Err(e) => DBIO::new(future::err(e)),
                }
            }
            #[cfg(feature = "mysql")]
            Self::Mysql(opts) => {
                use crate::connector::Mysql;

                match Mysql::new(opts.clone()) {
                    Ok(mysql) => DBIO::new(future::ok(Box::new(mysql) as Self::Resource)),
                    Err(e) => DBIO::new(future::err(e)),
                }
            }
            #[cfg(feature = "postgresql")]
            Self::Postgres {
                config,
                schema,
                ssl_params,
            } => {
                use crate::connector::PostgreSql;

                let config = config.clone();
                let schema = schema.clone();
                let ssl_params = ssl_params.clone();

                DBIO::new(async move {
                    let conn = PostgreSql::new(config, Some(schema), Some(ssl_params)).await?;

                    Ok(Box::new(conn) as Self::Resource)
                })
            }
        }
    }

    fn status(&self, _: &Self::Resource) -> Status {
        Status::Valid
    }

    fn recycle(&self, conn: Self::Resource) -> Self::RecycleFuture {
        DBIO::new(async {
            conn.query_raw("SELECT 1", &[]).await?;
            Ok(Some(conn))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Quaint;
    use std::env;

    #[test]
    #[cfg(feature = "mysql")]
    fn mysql_default_connection_limit() {
        let conn_string = env::var("TEST_MYSQL").expect("TEST_MYSQL connection string not set.");

        let pool = Quaint::new(&conn_string).unwrap();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity());
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn mysql_custom_connection_limit() {
        let conn_string = format!(
            "{}?connection_limit=10",
            env::var("TEST_MYSQL").expect("TEST_MYSQL connection string not set.")
        );

        let pool = Quaint::new(&conn_string).unwrap();

        assert_eq!(10, pool.capacity());
    }

    #[test]
    #[cfg(feature = "postgresql")]
    fn psql_default_connection_limit() {
        let conn_string = env::var("TEST_PSQL").expect("TEST_PSQL connection string not set.");

        let pool = Quaint::new(&conn_string).unwrap();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity());
    }

    #[test]
    #[cfg(feature = "postgresql")]
    fn psql_custom_connection_limit() {
        let conn_string = format!(
            "{}?connection_limit=10",
            env::var("TEST_PSQL").expect("TEST_PSQL connection string not set.")
        );

        let pool = Quaint::new(&conn_string).unwrap();

        assert_eq!(10, pool.capacity());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn test_default_connection_limit() {
        let conn_string = format!("file:db/test.db",);
        let pool = Quaint::new(&conn_string).unwrap();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn test_custom_connection_limit() {
        let conn_string = format!("file:db/test.db?connection_limit=10",);
        let pool = Quaint::new(&conn_string).unwrap();

        assert_eq!(10, pool.capacity());
    }
}
