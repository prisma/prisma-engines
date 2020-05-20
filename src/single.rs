//! A single connection abstraction to a SQL database.

use crate::{
    ast,
    connector::{self, ConnectionInfo, Queryable, SqlFamily, TransactionCapable},
};
use async_trait::async_trait;
use std::{fmt, sync::Arc};
use url::Url;

#[cfg(feature = "sqlite")]
use std::convert::TryFrom;

/// The main entry point and an abstraction over a database connection.
#[derive(Clone)]
pub struct Quaint {
    inner: Arc<dyn Queryable + Send + Sync>,
    connection_info: Arc<ConnectionInfo>,
}

impl fmt::Debug for Quaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.connection_info)
    }
}

impl TransactionCapable for Quaint {}

impl Quaint {
    /// Create a new connection to the database. The connection string
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
    /// SQLite:
    ///
    /// - `user`/`password` do not do anything and can be emitted.
    /// - `host` should point to the database file.
    /// - `db_name` parameter should give a name to the database attached for
    ///   query namespacing.
    /// - `socket_timeout` defined in seconds. Acts as the busy timeout in
    ///   SQLite. When set, queries that are waiting for a lock to be released
    ///   will return the `Timeout` error after the defined value.
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
    /// - `socket_timeout` defined in seconds. If set, a query will return a
    ///   `Timeout` error if it fails to resolve before given time.
    /// - `connect_timeout` defined in seconds (default: 5). Connecting to a
    ///   database will return a `ConnectTimeout` error if taking more than the
    ///   defined value.
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
    /// - `socket_timeout` defined in seconds. If set, a query will return a
    ///   `Timeout` error if it fails to resolve before given time.
    /// - `connect_timeout` defined in seconds (default: 5). Connecting to a
    ///   database will return a `ConnectTimeout` error if taking more than the
    ///   defined value.
    pub async fn new(url_str: &str) -> crate::Result<Self> {
        let url = Url::parse(url_str)?;

        let inner = match url.scheme() {
            #[cfg(feature = "sqlite")]
            "file" | "sqlite" => {
                let params = connector::SqliteParams::try_from(url_str)?;
                let mut sqlite = connector::Sqlite::new(&params.file_path)?;

                sqlite.attach_database(&params.db_name).await?;

                Arc::new(sqlite) as Arc<dyn Queryable + Send + Sync>
            }
            #[cfg(feature = "mysql")]
            "mysql" => {
                let url = connector::MysqlUrl::new(url)?;
                let mysql = connector::Mysql::new(url)?;

                Arc::new(mysql) as Arc<dyn Queryable + Send + Sync>
            }
            #[cfg(feature = "postgresql")]
            "postgres" | "postgresql" => {
                let url = connector::PostgresUrl::new(url)?;
                let psql = connector::PostgreSql::new(url).await?;

                Arc::new(psql) as Arc<dyn Queryable + Send + Sync>
            }
            _ => unimplemented!("Supported url schemes: file or sqlite, mysql, postgres or postgresql."),
        };

        let connection_info = Arc::new(ConnectionInfo::from_url(url_str)?);
        Self::log_start(connection_info.sql_family(), 1);

        Ok(Self { inner, connection_info })
    }

    /// Info about the connection and underlying database.
    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }

    fn log_start(family: SqlFamily, connection_limit: u32) {
        #[cfg(not(feature = "tracing-log"))]
        {
            info!("Starting a {} pool with {} connections.", family, connection_limit);
        }
        #[cfg(feature = "tracing-log")]
        {
            tracing::info!("Starting a {} pool with {} connections.", family, connection_limit);
        }
    }
}

#[async_trait]
impl Queryable for Quaint {
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
}
