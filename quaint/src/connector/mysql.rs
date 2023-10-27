mod conversion;
mod error;

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use lru_cache::LruCache;
use mysql_async::{
    self as my,
    prelude::{Query as _, Queryable as _},
};
use percent_encoding::percent_decode;
use std::{
    borrow::Cow,
    future::Future,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::sync::Mutex;
use url::{Host, Url};

pub use error::MysqlError;

/// The underlying MySQL driver. Only available with the `expose-drivers`
/// Cargo feature.
#[cfg(feature = "expose-drivers")]
pub use mysql_async;

use super::IsolationLevel;

/// A connector interface for the MySQL database.
#[derive(Debug)]
pub struct Mysql {
    pub(crate) conn: Mutex<my::Conn>,
    pub(crate) url: MysqlUrl,
    socket_timeout: Option<Duration>,
    is_healthy: AtomicBool,
    statement_cache: Mutex<LruCache<String, my::Statement>>,
}

/// Wraps a connection url and exposes the parsing logic used by quaint, including default values.
#[derive(Debug, Clone)]
pub struct MysqlUrl {
    url: Url,
    query_params: MysqlUrlQueryParams,
}

impl MysqlUrl {
    /// Parse `Url` to `MysqlUrl`. Returns error for mistyped connection
    /// parameters.
    pub fn new(url: Url) -> Result<Self, Error> {
        let query_params = Self::parse_query_params(&url)?;

        Ok(Self { url, query_params })
    }

    /// The bare `Url` to the database.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// The percent-decoded database username.
    pub fn username(&self) -> Cow<str> {
        match percent_decode(self.url.username().as_bytes()).decode_utf8() {
            Ok(username) => username,
            Err(_) => {
                tracing::warn!("Couldn't decode username to UTF-8, using the non-decoded version.");

                self.url.username().into()
            }
        }
    }

    /// The percent-decoded database password.
    pub fn password(&self) -> Option<Cow<str>> {
        match self
            .url
            .password()
            .and_then(|pw| percent_decode(pw.as_bytes()).decode_utf8().ok())
        {
            Some(password) => Some(password),
            None => self.url.password().map(|s| s.into()),
        }
    }

    /// Name of the database connected. Defaults to `mysql`.
    pub fn dbname(&self) -> &str {
        match self.url.path_segments() {
            Some(mut segments) => segments.next().unwrap_or("mysql"),
            None => "mysql",
        }
    }

    /// The database host. If `socket` and `host` are not set, defaults to `localhost`.
    pub fn host(&self) -> &str {
        match (self.url.host(), self.url.host_str()) {
            (Some(Host::Ipv6(_)), Some(host)) => {
                // The `url` crate may return an IPv6 address in brackets, which must be stripped.
                if host.starts_with('[') && host.ends_with(']') {
                    &host[1..host.len() - 1]
                } else {
                    host
                }
            }
            (_, Some(host)) => host,
            _ => "localhost",
        }
    }

    /// If set, connected to the database through a Unix socket.
    pub fn socket(&self) -> &Option<String> {
        &self.query_params.socket
    }

    /// The database port, defaults to `3306`.
    pub fn port(&self) -> u16 {
        self.url.port().unwrap_or(3306)
    }

    /// The connection timeout.
    pub fn connect_timeout(&self) -> Option<Duration> {
        self.query_params.connect_timeout
    }

    /// The pool check_out timeout
    pub fn pool_timeout(&self) -> Option<Duration> {
        self.query_params.pool_timeout
    }

    /// The socket timeout
    pub fn socket_timeout(&self) -> Option<Duration> {
        self.query_params.socket_timeout
    }

    /// Prefer socket connection
    pub fn prefer_socket(&self) -> Option<bool> {
        self.query_params.prefer_socket
    }

    /// The maximum connection lifetime
    pub fn max_connection_lifetime(&self) -> Option<Duration> {
        self.query_params.max_connection_lifetime
    }

    /// The maximum idle connection lifetime
    pub fn max_idle_connection_lifetime(&self) -> Option<Duration> {
        self.query_params.max_idle_connection_lifetime
    }

    fn statement_cache_size(&self) -> usize {
        self.query_params.statement_cache_size
    }

    pub(crate) fn cache(&self) -> LruCache<String, my::Statement> {
        LruCache::new(self.query_params.statement_cache_size)
    }

    fn parse_query_params(url: &Url) -> Result<MysqlUrlQueryParams, Error> {
        let mut ssl_opts = my::SslOpts::default();
        ssl_opts = ssl_opts.with_danger_accept_invalid_certs(true);

        let mut connection_limit = None;
        let mut use_ssl = false;
        let mut socket = None;
        let mut socket_timeout = None;
        let mut connect_timeout = Some(Duration::from_secs(5));
        let mut pool_timeout = Some(Duration::from_secs(10));
        let mut max_connection_lifetime = None;
        let mut max_idle_connection_lifetime = Some(Duration::from_secs(300));
        let mut prefer_socket = None;
        let mut statement_cache_size = 100;
        let mut identity: Option<(Option<PathBuf>, Option<String>)> = None;

        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "connection_limit" => {
                    let as_int: usize = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    connection_limit = Some(as_int);
                }
                "statement_cache_size" => {
                    statement_cache_size = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                }
                "sslcert" => {
                    use_ssl = true;
                    ssl_opts = ssl_opts.with_root_cert_path(Some(Path::new(&*v).to_path_buf()));
                }
                "sslidentity" => {
                    use_ssl = true;

                    identity = match identity {
                        Some((_, pw)) => Some((Some(Path::new(&*v).to_path_buf()), pw)),
                        None => Some((Some(Path::new(&*v).to_path_buf()), None)),
                    };
                }
                "sslpassword" => {
                    use_ssl = true;

                    identity = match identity {
                        Some((path, _)) => Some((path, Some(v.to_string()))),
                        None => Some((None, Some(v.to_string()))),
                    };
                }
                "socket" => {
                    socket = Some(v.replace(['(', ')'], ""));
                }
                "socket_timeout" => {
                    let as_int = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                    socket_timeout = Some(Duration::from_secs(as_int));
                }
                "prefer_socket" => {
                    let as_bool = v
                        .parse::<bool>()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                    prefer_socket = Some(as_bool)
                }
                "connect_timeout" => {
                    let as_int = v
                        .parse::<u64>()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    connect_timeout = match as_int {
                        0 => None,
                        _ => Some(Duration::from_secs(as_int)),
                    };
                }
                "pool_timeout" => {
                    let as_int = v
                        .parse::<u64>()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    pool_timeout = match as_int {
                        0 => None,
                        _ => Some(Duration::from_secs(as_int)),
                    };
                }
                "sslaccept" => {
                    use_ssl = true;
                    match v.as_ref() {
                        "strict" => {
                            ssl_opts = ssl_opts.with_danger_accept_invalid_certs(false);
                        }
                        "accept_invalid_certs" => {}
                        _ => {
                            tracing::debug!(
                                message = "Unsupported SSL accept mode, defaulting to `accept_invalid_certs`",
                                mode = &*v
                            );
                        }
                    };
                }
                "max_connection_lifetime" => {
                    let as_int = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    if as_int == 0 {
                        max_connection_lifetime = None;
                    } else {
                        max_connection_lifetime = Some(Duration::from_secs(as_int));
                    }
                }
                "max_idle_connection_lifetime" => {
                    let as_int = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    if as_int == 0 {
                        max_idle_connection_lifetime = None;
                    } else {
                        max_idle_connection_lifetime = Some(Duration::from_secs(as_int));
                    }
                }
                _ => {
                    tracing::trace!(message = "Discarding connection string param", param = &*k);
                }
            };
        }

        ssl_opts = match identity {
            Some((Some(path), Some(pw))) => {
                let identity = mysql_async::ClientIdentity::new(path).with_password(pw);
                ssl_opts.with_client_identity(Some(identity))
            }
            Some((Some(path), None)) => {
                let identity = mysql_async::ClientIdentity::new(path);
                ssl_opts.with_client_identity(Some(identity))
            }
            _ => ssl_opts,
        };

        Ok(MysqlUrlQueryParams {
            ssl_opts,
            connection_limit,
            use_ssl,
            socket,
            socket_timeout,
            connect_timeout,
            pool_timeout,
            max_connection_lifetime,
            max_idle_connection_lifetime,
            prefer_socket,
            statement_cache_size,
        })
    }

    #[cfg(feature = "pooled")]
    pub(crate) fn connection_limit(&self) -> Option<usize> {
        self.query_params.connection_limit
    }

    pub(crate) fn to_opts_builder(&self) -> my::OptsBuilder {
        let mut config = my::OptsBuilder::default()
            .stmt_cache_size(Some(0))
            .user(Some(self.username()))
            .pass(self.password())
            .db_name(Some(self.dbname()));

        match self.socket() {
            Some(ref socket) => {
                config = config.socket(Some(socket));
            }
            None => {
                config = config.ip_or_hostname(self.host()).tcp_port(self.port());
            }
        }

        config = config.conn_ttl(Some(Duration::from_secs(5)));

        if self.query_params.use_ssl {
            config = config.ssl_opts(Some(self.query_params.ssl_opts.clone()));
        }

        if self.query_params.prefer_socket.is_some() {
            config = config.prefer_socket(self.query_params.prefer_socket);
        }

        config
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MysqlUrlQueryParams {
    ssl_opts: my::SslOpts,
    connection_limit: Option<usize>,
    use_ssl: bool,
    socket: Option<String>,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    pool_timeout: Option<Duration>,
    max_connection_lifetime: Option<Duration>,
    max_idle_connection_lifetime: Option<Duration>,
    prefer_socket: Option<bool>,
    statement_cache_size: usize,
}

impl Mysql {
    /// Create a new MySQL connection using `OptsBuilder` from the `mysql` crate.
    pub async fn new(url: MysqlUrl) -> crate::Result<Self> {
        let conn = super::timeout::connect(url.connect_timeout(), my::Conn::new(url.to_opts_builder())).await?;

        Ok(Self {
            socket_timeout: url.query_params.socket_timeout,
            conn: Mutex::new(conn),
            statement_cache: Mutex::new(url.cache()),
            url,
            is_healthy: AtomicBool::new(true),
        })
    }

    /// The underlying mysql_async::Conn. Only available with the
    /// `expose-drivers` Cargo feature. This is a lower level API when you need
    /// to get into database specific features.
    #[cfg(feature = "expose-drivers")]
    pub fn conn(&self) -> &Mutex<mysql_async::Conn> {
        &self.conn
    }

    async fn perform_io<F, U, T>(&self, op: U) -> crate::Result<T>
    where
        F: Future<Output = crate::Result<T>>,
        U: FnOnce() -> F,
    {
        match super::timeout::socket(self.socket_timeout, op()).await {
            Err(e) if e.is_closed() => {
                self.is_healthy.store(false, Ordering::SeqCst);
                Err(e)
            }
            res => Ok(res?),
        }
    }

    async fn prepared<F, U, T>(&self, sql: &str, op: U) -> crate::Result<T>
    where
        F: Future<Output = crate::Result<T>>,
        U: Fn(my::Statement) -> F,
    {
        if self.url.statement_cache_size() == 0 {
            self.perform_io(|| async move {
                let stmt = {
                    let mut conn = self.conn.lock().await;
                    conn.prep(sql).await?
                };

                let res = op(stmt.clone()).await;

                {
                    let mut conn = self.conn.lock().await;
                    conn.close(stmt).await?;
                }

                res
            })
            .await
        } else {
            self.perform_io(|| async move {
                let stmt = self.fetch_cached(sql).await?;
                op(stmt).await
            })
            .await
        }
    }

    async fn fetch_cached(&self, sql: &str) -> crate::Result<my::Statement> {
        let mut cache = self.statement_cache.lock().await;
        let capacity = cache.capacity();
        let stored = cache.len();

        match cache.get_mut(sql) {
            Some(stmt) => {
                tracing::trace!(
                    message = "CACHE HIT!",
                    query = sql,
                    capacity = capacity,
                    stored = stored,
                );

                Ok(stmt.clone()) // arc'd
            }
            None => {
                tracing::trace!(
                    message = "CACHE MISS!",
                    query = sql,
                    capacity = capacity,
                    stored = stored,
                );

                let mut conn = self.conn.lock().await;
                if cache.capacity() == cache.len() {
                    if let Some((_, stmt)) = cache.remove_lru() {
                        conn.close(stmt).await?;
                    }
                }

                let stmt = conn.prep(sql).await?;
                cache.insert(sql.to_string(), stmt.clone());

                Ok(stmt)
            }
        }
    }
}

impl_default_TransactionCapable!(Mysql);

#[async_trait]
impl Queryable for Mysql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Mysql::build(q)?;
        self.query_raw(&sql, &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("mysql.query_raw", sql, params, move || async move {
            self.prepared(sql, |stmt| async move {
                let mut conn = self.conn.lock().await;
                let rows: Vec<my::Row> = conn.exec(&stmt, conversion::conv_params(params)?).await?;
                let columns = stmt.columns().iter().map(|s| s.name_str().into_owned()).collect();

                let last_id = conn.last_insert_id();
                let mut result_set = ResultSet::new(columns, Vec::new());

                for mut row in rows {
                    result_set.rows.push(row.take_result_row()?);
                }

                if let Some(id) = last_id {
                    result_set.set_last_insert_id(id);
                };

                Ok(result_set)
            })
            .await
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.query_raw(sql, params).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Mysql::build(q)?;
        self.execute_raw(&sql, &params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("mysql.execute_raw", sql, params, move || async move {
            self.prepared(sql, |stmt| async move {
                let mut conn = self.conn.lock().await;
                conn.exec_drop(stmt, conversion::conv_params(params)?).await?;

                Ok(conn.affected_rows())
            })
            .await
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("mysql.raw_cmd", cmd, &[], move || async move {
            self.perform_io(|| async move {
                let mut conn = self.conn.lock().await;
                let mut result = cmd.run(&mut *conn).await?;

                loop {
                    result.map(drop).await?;

                    if result.is_empty() {
                        result.map(drop).await?;
                        break;
                    }
                }

                Ok(())
            })
            .await
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT @@GLOBAL.version version"#;
        let rows = super::timeout::socket(self.socket_timeout, self.query_raw(query, &[])).await?;

        let version_string = rows
            .get(0)
            .and_then(|row| row.get("version").and_then(|version| version.typed.to_string()));

        Ok(version_string)
    }

    fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        if matches!(isolation_level, IsolationLevel::Snapshot) {
            return Err(Error::builder(ErrorKind::invalid_isolation_level(&isolation_level)).build());
        }

        self.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation_level}"))
            .await?;

        Ok(())
    }

    fn requires_isolation_first(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::MysqlUrl;
    use crate::tests::test_api::mysql::CONN_STR;
    use crate::{error::*, single::Quaint};
    use url::Url;

    #[test]
    fn should_parse_socket_url() {
        let url = MysqlUrl::new(Url::parse("mysql://root@localhost/dbname?socket=(/tmp/mysql.sock)").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!(&Some(String::from("/tmp/mysql.sock")), url.socket());
    }

    #[test]
    fn should_parse_prefer_socket() {
        let url =
            MysqlUrl::new(Url::parse("mysql://root:root@localhost:3307/testdb?prefer_socket=false").unwrap()).unwrap();
        assert!(!url.prefer_socket().unwrap());
    }

    #[test]
    fn should_parse_sslaccept() {
        let url =
            MysqlUrl::new(Url::parse("mysql://root:root@localhost:3307/testdb?sslaccept=strict").unwrap()).unwrap();
        assert!(url.query_params.use_ssl);
        assert!(!url.query_params.ssl_opts.skip_domain_validation());
        assert!(!url.query_params.ssl_opts.accept_invalid_certs());
    }

    #[test]
    fn should_parse_ipv6_host() {
        let url = MysqlUrl::new(Url::parse("mysql://[2001:db8:1234::ffff]:5432/testdb").unwrap()).unwrap();
        assert_eq!("2001:db8:1234::ffff", url.host());
    }

    #[test]
    fn should_allow_changing_of_cache_size() {
        let url = MysqlUrl::new(Url::parse("mysql:///root:root@localhost:3307/foo?statement_cache_size=420").unwrap())
            .unwrap();
        assert_eq!(420, url.cache().capacity());
    }

    #[test]
    fn should_have_default_cache_size() {
        let url = MysqlUrl::new(Url::parse("mysql:///root:root@localhost:3307/foo").unwrap()).unwrap();
        assert_eq!(100, url.cache().capacity());
    }

    #[tokio::test]
    async fn should_map_nonexisting_database_error() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.set_username("root").unwrap();
        url.set_path("/this_does_not_exist");

        let url = url.as_str().to_string();
        let res = Quaint::new(&url).await;

        let err = res.unwrap_err();

        match err.kind() {
            ErrorKind::DatabaseDoesNotExist { db_name } => {
                assert_eq!(Some("1049"), err.original_code());
                assert_eq!(Some("Unknown database \'this_does_not_exist\'"), err.original_message());
                assert_eq!(&Name::available("this_does_not_exist"), db_name)
            }
            e => panic!("Expected `DatabaseDoesNotExist`, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn should_map_wrong_credentials_error() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.set_username("WRONG").unwrap();

        let res = Quaint::new(url.as_str()).await;
        assert!(res.is_err());

        let err = res.unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::AuthenticationFailed { user } if user == &Name::available("WRONG")));
    }
}
