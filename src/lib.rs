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
//! use quaint::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), quaint::error::Error> {
//!     let quaint = Quaint::new("file:///tmp/example.db")?;
//!     let conn = quaint.check_out().await?;
//!     let result = conn.select(Select::default().value(1)).await?;
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
//! use quaint::{prelude::*, visitor::{Sqlite, Visitor}};
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
#[cfg(not(feature = "tracing-log"))]
#[macro_use]
extern crate log;

#[macro_use]
extern crate metrics;

#[macro_use]
extern crate debug_stub_derive;

pub mod ast;
pub mod connector;
pub mod error;
pub mod pool;
pub mod prelude;
pub mod visitor;

pub type Result<T> = std::result::Result<T, error::Error>;

use connector::{Queryable, DBIO};
use lazy_static::lazy_static;
use pool::{ConnectionInfo, PooledConnection, QuaintManager, SqlFamily};
use std::convert::TryFrom;
use tokio_resource_pool::{Builder, Pool};
use url::Url;

lazy_static! {
    static ref LOG_QUERIES: bool = std::env::var("LOG_QUERIES").map(|_| true).unwrap_or(false);
}

/// The main entry point and an abstraction over database connections and
/// pooling.
pub struct Quaint {
    pub inner: Pool<QuaintManager>,
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
    /// database. If not set, defaults to the [HikariCP
    /// Recommendation](https://github.com/brettwooldridge/HikariCP/wiki/About-Pool-Sizing):
    /// `physical_cpus * 2 + 1`.
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
            }
            #[cfg(feature = "mysql")]
            "mysql" => {
                let params = connector::MysqlParams::try_from(url)?;
                let manager = QuaintManager::Mysql(params.config);

                (manager, params.connection_limit)
            }
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
            _ => unimplemented!(
                "Supported url schemes: file or sqlite, mysql, postgres or postgresql."
            ),
        };

        let connection_info = ConnectionInfo::from_url(url_str)?;
        Self::log_start(connection_info.sql_family(), connection_limit);

        Ok(Self {
            inner: Builder::new().build(connection_limit as usize, manager),
            connection_info,
        })
    }

    /// The number of connections in the pool.
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

    fn log_start(family: SqlFamily, connection_limit: u32) {
        #[cfg(not(feature = "tracing-log"))]
        {
            info!(
                "Starting a {} pool with {} connections.",
                family, connection_limit
            );
        }
        #[cfg(feature = "tracing-log")]
        {
            tracing::info!(
                "Starting a {} pool with {} connections.",
                family,
                connection_limit
            );
        }
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

    fn execute_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ast::ParameterizedValue],
    ) -> DBIO<'a, u64> {
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

    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()> {
        DBIO::new(async move {
            let conn = self.check_out().await?;
            conn.raw_cmd(cmd).await
        })
    }
}
