//! # A connection pool to a SQL database.
//!
//! A pool is created through the [`builder`] method, starting from a connection
//! string that allows some of the parameters be delivered by the user.
//!
//! A connection string has the following structure:
//!
//! `connector_type://user:password@host/database?parameters`
//!
//! Connector type can be one of the following:
//!
//! - `sqlite`/`file` opens an SQLite connection
//! - `mysql` opens a MySQL connection
//! - `postgres`/`postgresql` opens a PostgreSQL connection
//!
//! All parameters should be given in the query string format:
//! `?key1=val1&key2=val2`. All parameters are optional.
//!
//! ## Common parameters
//!
//! - `connection_limit` defines the maximum number of connections opened to the
//!   database.
//!
//! ## SQLite
//!
//! - `user`/`password` do not do anything and can be emitted.
//! - `host` should point to the database file.
//! - `db_name` parameter should give a name to the database attached for
//!   query namespacing.
//! - `socket_timeout` defined in seconds. Acts as the busy timeout in
//!   SQLite. When set, queries that are waiting for a lock to be released
//!   will return the `Timeout` error after the defined value.
//!
//! ## PostgreSQL
//!
//! - `sslmode` either `disable`, `prefer` or `require`. [Read more](https://docs.rs/tokio-postgres/0.5.0-alpha.1/tokio_postgres/config/enum.SslMode.html)
//! - `sslcert` should point to a PEM certificate file.
//! - `sslidentity` should point to a PKCS12 certificate database.
//! - `sslpassword` the password to open the PKCS12 database.
//! - `sslaccept` either `strict` or `accept_invalid_certs`. If strict, the
//!   certificate needs to be valid and in the CA certificates.
//!   `accept_invalid_certs` accepts any certificate from the server and can
//!   lead to weakened security. Defaults to `accept_invalid_certs`.
//! - `schema` the default search path.
//! - `host` additionally the host can be given as a parameter, typically in
//!   cases when connectiong to the database through a unix socket to
//!   separate the database name from the database path, such as
//!   `postgresql:///dbname?host=/var/run/postgresql`.
//! - `socket_timeout` defined in seconds. If set, a query will return a
//!   `Timeout` error if it fails to resolve before given time.
//! - `connect_timeout` defined in seconds (default: 5). Connecting to a
//!   database will return a `ConnectTimeout` error if taking more than the
//!   defined value.
//! - `pgbouncer` either `true` or `false`. If set, allows usage with the
//!   pgBouncer connection pool in transaction mode. Additionally a transaction
//!   is required for every query for the mode to work. When starting a new
//!   transaction, a deallocation query `DEALLOCATE ALL` is executed right after
//!   `BEGIN` to avoid possible collisions with statements created in other
//!   sessions.
//!
//! ## MySQL
//!
//! - `sslcert` should point to a PEM certificate file.
//! - `sslidentity` should point to a PKCS12 certificate database.
//! - `sslpassword` the password to open the PKCS12 database.
//! - `sslaccept` either `strict` or `accept_invalid_certs`. If strict, the
//!   certificate needs to be valid and in the CA certificates.
//!   `accept_invalid_certs` accepts any certificate from the server and can
//!   lead to weakened security. Defaults to `strict`.
//! - `socket` needed when connecting to MySQL database through a unix
//!   socket. When set, the host parameter is dismissed.
//! - `socket_timeout` defined in seconds. If set, a query will return a
//!   `Timeout` error if it fails to resolve before given time.
//! - `connect_timeout` defined in seconds (default: 5). Connecting to a
//!   database will return a `ConnectTimeout` error if taking more than the
//!   defined value.
//!
//! To create a new `Quaint` pool connecting to a PostgreSQL database:
//!
//! ``` no_run
//! use quaint::{prelude::*, pooled::Quaint};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), quaint::error::Error> {
//!     let mut builder = Quaint::builder("postgresql://postgres:password@localhost:5432/postgres")?;
//!     builder.connection_limit(5);
//!     builder.connect_timeout(Duration::from_secs(5));
//!     builder.max_idle_lifetime(Duration::from_secs(300));
//!     builder.test_on_check_out(true);
//!
//!     let pool = builder.build();
//!     let conn = pool.check_out().await?;
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
//! [`builder`]: struct.Quaint.html#method.builder

mod manager;

pub use manager::*;

use crate::connector::ConnectionInfo;
use mobc::Pool;
use std::{sync::Arc, time::Duration};
use url::Url;

#[cfg(feature = "sqlite")]
use std::convert::TryFrom;

/// The main entry point and an abstraction over database connections and
/// connection handling.
#[derive(Clone)]
pub struct Quaint {
    pub(crate) inner: Pool<QuaintManager>,
    connection_info: Arc<ConnectionInfo>,
    connect_timeout: Option<Duration>,
}

/// A `Builder` to construct an instance of a [`Quaint`] pool.
///
/// [`Quaint`]: pooled.Quaint
pub struct Builder {
    manager: QuaintManager,
    connection_info: ConnectionInfo,
    connection_limit: usize,
    max_idle: Option<u64>,
    max_idle_lifetime: Option<Duration>,
    health_check_interval: Option<Duration>,
    test_on_check_out: bool,
    connect_timeout: Option<Duration>,
}

impl Builder {
    fn new(url: &str, manager: QuaintManager) -> crate::Result<Self> {
        let connection_limit = num_cpus::get_physical() * 2 + 1;
        let connection_info = ConnectionInfo::from_url(url)?;

        Ok(Self {
            manager,
            connection_info,
            connection_limit,
            max_idle: None,
            max_idle_lifetime: None,
            health_check_interval: None,
            test_on_check_out: false,
            connect_timeout: None,
        })
    }

    /// The maximum number of connections in the pool.
    ///
    /// - Defaults to two times the number of physical cores plus one.
    pub fn connection_limit(&mut self, connection_limit: usize) {
        self.connection_limit = connection_limit;
    }

    /// The maximum number of idle connections the pool can contain at the same time. If a
    /// connection goes idle (a query returns) and there are already this number of idle connections
    /// in the pool, a connection will be closed immediately. Consider using `max_idle_lifetime` to
    /// close idle connections less aggressively.
    ///
    /// - Defaults to the same value as `connection_limit`.
    pub fn max_idle(&mut self, max_idle: u64) {
        self.max_idle = Some(max_idle);
    }

    /// A timeout for acquiring a connection with the [`check_out`] method. If
    /// not set, the method never times out.
    ///
    /// # Panics
    ///
    /// Panics if `connect_timeout` is zero.
    ///
    /// [`check_out`]: struct.Quaint.html#method.check_out
    pub fn connect_timeout(&mut self, connect_timeout: Duration) {
        assert_ne!(
            connect_timeout,
            Duration::from_secs(0),
            "connect_timeout must be positive"
        );

        self.connect_timeout = Some(connect_timeout);
    }

    /// A time how long an idling connection can be kept in the pool before
    /// replaced with a new one. The reconnect happens in the next
    /// [`check_out`].
    ///
    /// - Defaults to not set, meaning idle connections are never reconnected.
    ///
    /// # Panics
    ///
    /// Panics if `max_idle_lifetime` is zero.
    ///
    /// [`check_out`]: struct.Quaint.html#method.check_out
    pub fn max_idle_lifetime(&mut self, max_idle_lifetime: Duration) {
        self.max_idle_lifetime = Some(max_idle_lifetime);
    }

    /// Perform a health check before returning a connection from the
    /// [`check_out`]. If the health check fails, a few reconnects are tried
    /// before returning the error and dropping the broken connection from the
    /// pool.
    ///
    /// - Defaults to `false`, meaning connections are never tested on
    /// `check_out`.
    ///
    /// [`check_out`]: struct.Quaint.html#method.check_out
    pub fn test_on_check_out(&mut self, test_on_check_out: bool) {
        self.test_on_check_out = test_on_check_out;
    }

    /// Sets the interval how often a connection health will be tested when
    /// checking out from the pool. Must be used together with
    /// [`test_on_check_out`] set to `true`, otherwise does nothing.
    ///
    /// - Defaults to not set, meaning a test is performed on every `check_out`.
    ///
    /// # Panics
    ///
    /// Panics if `health_check_interval` is zero.
    ///
    /// [`test_on_check_out`]: #method.test_on_check_out
    pub fn health_check_interval(&mut self, health_check_interval: Duration) {
        self.health_check_interval = Some(health_check_interval);
    }

    /// Consume the builder and create a new instance of a pool.
    pub fn build(self) -> Quaint {
        let connection_info = Arc::new(self.connection_info);
        let family = connection_info.sql_family();

        #[cfg(not(feature = "tracing-log"))]
        {
            info!(
                "Starting a {} pool with up to {} connections.",
                family, self.connection_limit
            );
        }
        #[cfg(feature = "tracing-log")]
        {
            tracing::info!(
                "Starting a {} pool with up to {} connections.",
                family,
                self.connection_limit
            );
        }

        let inner = Pool::builder()
            .max_open(self.connection_limit as u64)
            .max_idle(self.max_idle.unwrap_or(self.connection_limit as u64))
            .max_idle_lifetime(self.max_idle_lifetime)
            .health_check_interval(self.health_check_interval)
            .test_on_check_out(self.test_on_check_out)
            .build(self.manager);

        Quaint {
            inner,
            connection_info,
            connect_timeout: self.connect_timeout,
        }
    }
}

impl Quaint {
    /// Creates a new builder for a Quaint connection pool with the given
    /// connection string. See the [module level documentation] for details.
    ///
    /// [module level documentation]: index.html
    pub fn builder(url_str: &str) -> crate::Result<Builder> {
        let url = Url::parse(url_str)?;

        match url.scheme() {
            #[cfg(feature = "sqlite")]
            "file" | "sqlite" => {
                let params = crate::connector::SqliteParams::try_from(url_str)?;

                let manager = QuaintManager::Sqlite {
                    file_path: params.file_path,
                    db_name: params.db_name,
                };

                let mut builder = Builder::new(url_str, manager)?;

                if let Some(limit) = params.connection_limit {
                    builder.connection_limit(limit);
                }

                Ok(builder)
            }
            #[cfg(feature = "mysql")]
            "mysql" => {
                let url = crate::connector::MysqlUrl::new(url)?;
                let connection_limit = url.connection_limit();
                let connect_timeout = url.connect_timeout();

                let manager = QuaintManager::Mysql(url);
                let mut builder = Builder::new(url_str, manager)?;

                if let Some(limit) = connection_limit {
                    builder.connection_limit(limit);
                }

                if let Some(timeout) = connect_timeout {
                    builder.connect_timeout(timeout);
                }

                Ok(builder)
            }
            #[cfg(feature = "postgresql")]
            "postgres" | "postgresql" => {
                let url = crate::connector::PostgresUrl::new(url)?;
                let connection_limit = url.connection_limit();
                let connect_timeout = url.connect_timeout();

                let manager = QuaintManager::Postgres(url);
                let mut builder = Builder::new(url_str, manager)?;

                if let Some(limit) = connection_limit {
                    builder.connection_limit(limit);
                }

                if let Some(timeout) = connect_timeout {
                    builder.connect_timeout(timeout);
                }

                Ok(builder)
            }
            _ => unimplemented!("Supported url schemes: file or sqlite, mysql, postgres or postgresql."),
        }
    }

    /// The number of connections in the pool.
    pub async fn capacity(&self) -> u32 {
        self.inner.state().await.max_open as u32
    }

    /// Reserve a connection from the pool.
    pub async fn check_out(&self) -> crate::Result<PooledConnection> {
        let inner = match self.connect_timeout {
            Some(duration) => self.inner.get_timeout(duration).await?,
            None => self.inner.get().await?,
        };

        Ok(PooledConnection { inner })
    }

    /// Info about the connection and underlying database.
    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }
}
