mod connection_info;

pub use connection_info::*;

use url::Url;
use crate::{
    ast,
    connector::{self, DBIO, Queryable},
    error::Error,
};
use tokio_resource_pool::{Builder, Pool, Status, CheckOut, Manage, RealDependencies};
use futures::future;
use std::convert::TryFrom;

/// The main entry point and an abstraction over database connections and
/// pooling.
pub struct Quaint {
    inner: Pool<QuaintManager>,
    connection_info: ConnectionInfo,
}

impl Quaint {
    /// Create a new pool of connections to the database. The connection string
    /// follows the specified format:
    ///
    /// `connector_type://user:password@host/database?parameters`
    ///
    /// Connector type can be one of the following:
    ///
    /// - `sqlite`/`file` opens an SQLite connection
    /// - `mysql` opens a MySQL connection
    /// - `postgres`/`postgresql` opens a PostgreSQL connection
    ///
    /// All parameters should be given in the query string format:
    /// `?key1=val1&key2=val2`. All parameters are optional.
    ///
    /// Common parameters:
    ///
    /// - `connection_limit` defines the number of connections opened to the
    /// database.
    ///
    /// SQLite:
    ///
    /// - `user`/`password` do not do anything and can be emitted.
    /// - `host` should point to the database file.
    /// - `db_name` parameter should give a name to the database attached for
    ///   query namespacing.
    ///
    /// PostgreSQL:
    ///
    /// - `sslmode` either `disable`, `prefer` or `require`. [Read more](https://docs.rs/tokio-postgres/0.5.0-alpha.1/tokio_postgres/config/enum.SslMode.html)
    /// - `sslcert` should point to a PEM certificate file.
    /// - `sslidentity` should point to a PKCS12 certificate database.
    /// - `sslpassword` the password to open the PKCS12 database.
    /// - `sslaccept` either `strict` or `accept_invalid_certs`. If strict, the
    ///   certificate needs to be valid and in the CA certificates.
    ///   `accept_invalid_certs` accepts any certificate from the server and can
    ///   lead to weakened security. Defaults to `strict`.
    /// - `schema` the default search path.
    /// - `host` additionally the host can be given as a parameter, typically in
    ///   cases when connectiong to the database through a unix socket to
    ///   separate the database name from the database path, such as
    ///   `postgresql:///dbname?host=/var/run/postgresql`.
    ///
    /// MySQL:
    ///
    /// - `sslcert` should point to a PEM certificate file.
    /// - `sslidentity` should point to a PKCS12 certificate database.
    /// - `sslpassword` the password to open the PKCS12 database.
    /// - `sslaccept` either `strict` or `accept_invalid_certs`. If strict, the
    ///   certificate needs to be valid and in the CA certificates.
    ///   `accept_invalid_certs` accepts any certificate from the server and can
    ///   lead to weakened security. Defaults to `strict`.
    /// - `socket` needed when connecting to MySQL database through a unix
    ///   socket. When set, the host parameter is dismissed.
    pub fn new(url_str: &str) -> crate::Result<Self> {
        let url = Url::parse(dbg!(url_str))?;

        let (manager, connection_limit) = match url.scheme() {
            #[cfg(feature = "sqlite")]
            "file" | "sqlite" => {
                let params = connector::SqliteParams::try_from(url_str)?;

                let manager = QuaintManager::Sqlite {
                    file_path: params.file_path,
                    db_name: params.db_name,
                };

                (manager, params.connection_limit)
            },
            #[cfg(feature = "mysql")]
            "mysql" => {
                let params = connector::MysqlParams::try_from(url)?;
                let manager = QuaintManager::Mysql(params.config);

                (manager, params.connection_limit)
            },
            #[cfg(feature = "postgresql")]
            "postgres" | "postgresql" => {
                let params = connector::PostgresParams::try_from(url)?;

                let manager = QuaintManager::Postgres {
                    config: params.config,
                    schema: params.schema,
                    ssl_params: params.ssl_params,
                };

                (manager, params.connection_limit)
            }
            _ => { unimplemented!("Supported url schemes: file or sqlite, mysql, postgres or postgresql.") }
        };

        #[cfg(not(feature = "tracing-log"))]
        {
            info!(
                "Starting a Quaint pool with {} connections.",
                connection_limit,
            );
        }
        #[cfg(feature = "tracing-log")]
        {
            tracing::info!(
                "Starting a Quaint pool with {} connections.",
                connection_limit,
            )
        }

        Ok(Self {
            inner: Builder::new().build(connection_limit as usize, manager),
            connection_info: ConnectionInfo::from_url(url_str)?,
        })
    }

    /// The number of capacity in the pool of connections.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Reserve a connection from the pool.
    pub async fn check_out(&self) -> crate::Result<PooledConnection> {
        Ok(PooledConnection {
            inner: self.inner.check_out().await?,
        })
    }

    /// Info about the connection and underlying database.
    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }
}

impl Queryable for Quaint {
    fn execute<'a>(&'a self, q: ast::Query<'a>) -> DBIO<'a, Option<ast::Id>> {
        DBIO::new(async move {
            let conn = self.check_out().await?;
            conn.execute(q).await
        })
    }

    fn query<'a>(&'a self, q: ast::Query<'a>) -> DBIO<'a, connector::ResultSet> {
        DBIO::new(async move {
            let conn = self.check_out().await?;
            conn.query(q).await
        })
    }

    fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ast::ParameterizedValue],
    ) -> DBIO<'a, connector::ResultSet> {
        DBIO::new(async move {
            let conn = self.check_out().await?;
            conn.query_raw(sql, params).await
        })
    }

    fn execute_raw<'a>(&'a self, sql: &'a str, params: &'a [ast::ParameterizedValue]) -> DBIO<'a, u64> {
        DBIO::new(async move {
            let conn = self.check_out().await?;
            conn.execute_raw(sql, params).await
        })
    }

    fn turn_off_fk_constraints(&self) -> DBIO<()> {
        DBIO::new(async move {
            let conn = self.check_out().await?;
            conn.turn_off_fk_constraints().await
        })
    }

    fn turn_on_fk_constraints(&self) -> DBIO<()> {
        DBIO::new(async move {
            let conn = self.check_out().await?;
            conn.turn_on_fk_constraints().await
        })
    }

    /// Please reserve a connection using
    /// [check_out](struct.Quaint.html#method.check_out). This method panics.
    fn start_transaction(&self) -> DBIO<connector::Transaction> {
        unimplemented!("Start the transaction by reserving a connection with `check_out`.")
    }

    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()> {
        DBIO::new(async move {
            let conn = self.check_out().await?;
            conn.raw_cmd(cmd).await
        })
    }
}

/// A connection from the pool. Implements
/// [Queryable](connector/trait.Queryable.html).
pub struct PooledConnection {
    inner: CheckOut<QuaintManager>,
}

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

    fn execute_raw<'a>(&'a self, sql: &'a str, params: &'a [ast::ParameterizedValue]) -> DBIO<'a, u64> {
        self.inner.execute_raw(sql, params)
    }

    fn turn_off_fk_constraints(&self) -> DBIO<()> {
        self.inner.turn_off_fk_constraints()
    }

    fn turn_on_fk_constraints(&self) -> DBIO<()> {
        self.inner.turn_on_fk_constraints()
    }

    fn start_transaction(&self) -> DBIO<connector::Transaction> {
        self.inner.start_transaction()
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
        db_name: Option<String>,
    }
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
                        match db_name {
                            Some(ref name) => {
                                match conn.attach_database(name) {
                                    Ok(_) => DBIO::new(future::ok(Box::new(conn) as Self::Resource)),
                                    Err(e) => DBIO::new(future::err(e)),
                                }
                            }
                            None => {
                                DBIO::new(future::ok(Box::new(conn) as Self::Resource))
                            }
                        }
                    }
                    Err(e) => DBIO::new(future::err(e))
                }
            }
            #[cfg(feature = "mysql")]
            Self::Mysql(opts) => {
                use crate::connector::Mysql;

                match Mysql::new(opts.clone()) {
                    Ok(mysql) => DBIO::new(future::ok(Box::new(mysql) as Self::Resource)),
                    Err(e) => DBIO::new(future::err(e)),
                }
            },
            #[cfg(feature = "postgresql")]
            Self::Postgres { config, schema, ssl_params, } => {
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
    use std::env;
    use super::Quaint;

    #[test]
    #[cfg(feature = "mysql")]
    fn mysql_default_connection_limit() {
        let conn_string = env::var("TEST_MYSQL")
            .expect("TEST_MYSQL connection string not set.");

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
        let conn_string = env::var("TEST_PSQL")
            .expect("TEST_PSQL connection string not set.");

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
