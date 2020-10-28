mod conversion;
mod error;

use async_trait::async_trait;
use mysql_async::{
    self as my,
    prelude::{Query as _, Queryable as _},
    Conn,
};
use percent_encoding::percent_decode;
use std::{borrow::Cow, future::Future, path::Path, time::Duration};
use tokio::time::timeout;
use url::Url;

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};

/// A connector interface for the MySQL database.
#[derive(Debug)]
#[cfg_attr(feature = "docs", doc(cfg(feature = "mysql")))]
pub struct Mysql {
    pub(crate) pool: my::Pool,
    pub(crate) url: MysqlUrl,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
}

/// Wraps a connection url and exposes the parsing logic used by quaint, including default values.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "docs", doc(cfg(feature = "mysql")))]
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
                #[cfg(not(feature = "tracing-log"))]
                warn!("Couldn't decode username to UTF-8, using the non-decoded version.");
                #[cfg(feature = "tracing-log")]
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
        self.url.host_str().unwrap_or("localhost")
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

    fn parse_query_params(url: &Url) -> Result<MysqlUrlQueryParams, Error> {
        let mut connection_limit = None;
        let mut ssl_opts = my::SslOpts::default();
        let mut use_ssl = false;
        let mut socket = None;
        let mut socket_timeout = None;
        let mut connect_timeout = None;

        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "connection_limit" => {
                    let as_int: usize = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    connection_limit = Some(as_int);
                }
                "sslcert" => {
                    use_ssl = true;
                    ssl_opts = ssl_opts.with_root_cert_path(Some(Path::new(&*v).to_path_buf()));
                }
                "sslidentity" => {
                    use_ssl = true;
                    ssl_opts = ssl_opts.with_pkcs12_path(Some(Path::new(&*v).to_path_buf()));
                }
                "sslpassword" => {
                    use_ssl = true;
                    ssl_opts = ssl_opts.with_password(Some(v.to_string()));
                }
                "socket" => {
                    socket = Some(v.replace("(", "").replace(")", ""));
                }
                "socket_timeout" => {
                    let as_int = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                    socket_timeout = Some(Duration::from_secs(as_int));
                }
                "connect_timeout" => {
                    let as_int = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                    connect_timeout = Some(Duration::from_secs(as_int));
                }
                "sslaccept" => {
                    match v.as_ref() {
                        "strict" => {}
                        "accept_invalid_certs" => {
                            ssl_opts = ssl_opts.with_danger_accept_invalid_certs(true);
                        }
                        _ => {
                            #[cfg(not(feature = "tracing-log"))]
                            debug!("Unsupported SSL accept mode {}, defaulting to `strict`", v);
                            #[cfg(feature = "tracing-log")]
                            tracing::debug!(
                                message = "Unsupported SSL accept mode, defaulting to `strict`",
                                mode = &*v
                            );
                        }
                    };
                }
                _ => {
                    #[cfg(not(feature = "tracing-log"))]
                    trace!("Discarding connection string param: {}", k);
                    #[cfg(feature = "tracing-log")]
                    tracing::trace!(message = "Discarding connection string param", param = &*k);
                }
            };
        }

        Ok(MysqlUrlQueryParams {
            ssl_opts,
            connection_limit,
            use_ssl,
            socket,
            connect_timeout,
            socket_timeout,
        })
    }

    #[cfg(feature = "pooled")]
    pub(crate) fn connection_limit(&self) -> Option<usize> {
        self.query_params.connection_limit
    }

    pub(crate) fn to_opts_builder(&self) -> my::OptsBuilder {
        let mut config = my::OptsBuilder::default()
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

        config = config.stmt_cache_size(Some(1000));
        config = config.conn_ttl(Some(Duration::from_secs(5)));

        if self.query_params.use_ssl {
            config = config.ssl_opts(Some(self.query_params.ssl_opts.clone()));
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
}

impl Mysql {
    /// Create a new MySQL connection using `OptsBuilder` from the `mysql` crate.
    pub fn new(url: MysqlUrl) -> crate::Result<Self> {
        let mut opts = url.to_opts_builder();
        let pool_opts = my::PoolOpts::default().with_constraints(my::PoolConstraints::new(1, 1).unwrap());
        opts = opts.pool_opts(pool_opts);

        Ok(Self {
            socket_timeout: url.query_params.socket_timeout,
            connect_timeout: url.query_params.connect_timeout,
            pool: my::Pool::new(opts),
            url,
        })
    }

    async fn timeout<T, F, E>(&self, f: F) -> crate::Result<T>
    where
        F: Future<Output = std::result::Result<T, E>>,
        E: Into<Error>,
    {
        match self.socket_timeout {
            Some(duration) => match timeout(duration, f).await {
                Ok(Ok(result)) => Ok(result),
                Ok(Err(err)) => Err(err.into()),
                Err(to) => Err(to.into()),
            },
            None => match f.await {
                Ok(result) => Ok(result),
                Err(err) => Err(err.into()),
            },
        }
    }

    async fn get_conn(&self) -> crate::Result<Conn> {
        match self.connect_timeout {
            Some(duration) => Ok(timeout(duration, self.pool.get_conn()).await??),
            None => Ok(self.pool.get_conn().await?),
        }
    }
}

impl TransactionCapable for Mysql {}

#[async_trait]
impl Queryable for Mysql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Mysql::build(q)?;
        self.query_raw(&sql, &params).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Mysql::build(q)?;
        self.execute_raw(&sql, &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("mysql.query_raw", sql, params, move || async move {
            let mut conn = self.get_conn().await?;
            let stmt = self.timeout(conn.prep(sql)).await?;
            let rows: Vec<my::Row> = self.timeout(conn.exec(&stmt, conversion::conv_params(params)?)).await?;

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
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("mysql.execute_raw", sql, params, move || async move {
            let mut conn = self.get_conn().await?;
            self.timeout(conn.exec_drop(sql, conversion::conv_params(params)?))
                .await?;
            Ok(conn.affected_rows())
        })
        .await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("mysql.raw_cmd", cmd, &[], move || async move {
            let mut conn = self.get_conn().await?;

            let fut = async {
                let mut result = cmd.run(&mut conn).await?;

                loop {
                    result.map(drop).await?;

                    if result.is_empty() {
                        result.map(drop).await?;
                        break;
                    }
                }

                crate::Result::<()>::Ok(())
            };

            self.timeout(fut).await?;

            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT @@GLOBAL.version version"#;
        let rows = self.query_raw(query, &[]).await?;

        let version_string = rows
            .get(0)
            .and_then(|row| row.get("version").and_then(|version| version.to_string()));

        Ok(version_string)
    }
}

#[cfg(test)]
mod tests {
    use super::MysqlUrl;
    use crate::tests::test_api::mysql::CONN_STR;
    use crate::{connector::Queryable, error::*, single::Quaint};
    use url::Url;

    #[test]
    fn should_parse_socket_url() {
        let url = MysqlUrl::new(Url::parse("mysql://root@localhost/dbname?socket=(/tmp/mysql.sock)").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!(&Some(String::from("/tmp/mysql.sock")), url.socket());
    }

    #[tokio::test]
    async fn should_map_nonexisting_database_error() {
        let mut url = Url::parse(&*CONN_STR).unwrap();
        url.set_username("root").unwrap();
        url.set_path("/this_does_not_exist");

        let url = url.as_str().to_string();
        let conn = Quaint::new(&url).await.unwrap();
        let res = conn.query_raw("SELECT 1 + 1", &[]).await;

        assert!(&res.is_err());

        let err = res.unwrap_err();

        match err.kind() {
            ErrorKind::DatabaseDoesNotExist { db_name } => {
                assert_eq!(Some("1049"), err.original_code());
                assert_eq!(Some("Unknown database \'this_does_not_exist\'"), err.original_message());
                assert_eq!("this_does_not_exist", db_name.as_str())
            }
            e => panic!("Expected `DatabaseDoesNotExist`, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn should_map_wrong_credentials_error() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.set_username("WRONG").unwrap();

        let conn = Quaint::new(url.as_str()).await.unwrap();
        let res = conn.query_raw("SELECT 1", &[]).await;
        assert!(res.is_err());

        let err = res.unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::AuthenticationFailed { user } if user == "WRONG"));
    }
}
