//! # quaint
//!
//! Quaint is an AST and database-specific visitors for creating SQL
//! statements.
//!
//! Under construction and will go through several rounds of changes. Not meant
//! for production use in the current form.
//!
//! ### Goals
//!
//! - Query generation when the database and conditions are not known beforehand.
//! - Parameterized queries when possible.
//! - A modular design, separate AST for query building and visitors for
//!   different databases.
//! - Database support behind a feature flag.
//!
//! ### Non-goals
//!
//! - Database-level type-safety in query building or being an ORM.
//!
//! ## Databases
//!
//! - SQLite
//! - PostgreSQL
//! - MySQL
//!
//! ## Examples
//!
//! ### Querying a database with an AST object
//!
//! The [Quaint](struct.Quaint.html) abstracts a generic pooling and connection
//! interface over different databases. It offers querying with the
//! [ast](ast/index.html) module or directly using raw strings. See
//! documentation for [Queryable](connector/trait.Queryable.html) for details.
//!
//! When querying with an ast object the queries are paremeterized
//! automatically.
//!
//! ```
//! use quaint::{ast::*, Quaint};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), quaint::error::Error> {
//!     let quaint = Quaint::new("sqlite:///tmp/test.db")?;
//!     let conn = quaint.check_out().await?;
//!
//!     let query = Select::default().value(1);
//!     let result = conn.query(query.into()).await?;
//!
//!     assert_eq!(
//!         Some(1),
//!         result.into_iter().nth(0).and_then(|row| row[0].as_i64()),
//!     );
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Building an SQL query string
//!
//! The crate can be used as an SQL string builder using the [ast](ast/index.html) and
//! [visitor](visitor/index.html) modules.
//!
//! AST is generic for all databases and the visitors generate correct SQL
//! syntax for the database.
//!
//! The visitor returns the query as a string and its parameters as a vector.
//!
//! ```
//! use quaint::{ast::*, visitor::{Sqlite, Visitor}};
//!
//! fn main() {
//!     let conditions = "word"
//!         .equals("meow")
//!         .and("age".less_than(10))
//!         .and("paw".equals("warm"));
//!
//!     let query = Select::from_table("naukio").so_that(conditions);
//!     let (sql_str, params) = Sqlite::build(query);
//!
//!     assert_eq!(
//!         "SELECT `naukio`.* FROM `naukio` WHERE ((`word` = ? AND `age` < ?) AND `paw` = ?)",
//!         sql_str,
//!     );
//!
//!     assert_eq!(
//!         vec![
//!             ParameterizedValue::from("meow"),
//!             ParameterizedValue::from(10),
//!             ParameterizedValue::from("warm"),
//!         ],
//!         params
//!     );
//! }
//! ```
pub mod ast;
pub mod connector;
pub mod error;

pub mod visitor;

#[cfg(not(feature = "tracing-log"))]
#[macro_use]
extern crate log;

#[macro_use]
extern crate metrics;

#[macro_use]
extern crate debug_stub_derive;

pub type Result<T> = std::result::Result<T, error::Error>;

use lazy_static::lazy_static;
use url::Url;
use connector::{DBIO, Queryable};
use error::Error;
use tokio_resource_pool::{Builder, Pool, Status, CheckOut, Manage, RealDependencies};
use futures::future;
use std::{convert::TryFrom, ops::Deref};

lazy_static! {
    static ref LOG_QUERIES: bool = std::env::var("LOG_QUERIES")
        .map(|_| true)
        .unwrap_or(false);
}

/// The main entry point and an abstraction over database connections and
/// pooling.
pub struct Quaint {
    inner: Pool<QuaintManager>,
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
    /// - `postgres` / `postgresql` opens a PostgreSQL connection
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
        let url = Url::parse(url_str)?;

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
}

/// A connection from the pool. Implements
/// [Queryable](connector/trait.Queryable.html).
pub struct PooledConnection {
    inner: CheckOut<QuaintManager>,
}

impl Deref for PooledConnection {
    type Target = Box<dyn Queryable + Send + Sync + 'static>;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

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
    type Resource = Box<dyn Queryable + Send + Sync + 'static>;
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

                DBIO::new(match Mysql::new(opts.clone()) {
                    Ok(mysql) => future::ok(Box::new(mysql) as Self::Resource),
                    Err(e) => future::err(e),
                })
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
