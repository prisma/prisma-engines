use crate::error::{Error, ErrorKind};
use percent_encoding::percent_decode;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    time::Duration,
};
use url::{Host, Url};

/// Wraps a connection url and exposes the parsing logic used by quaint, including default values.
#[derive(Debug, Clone)]
pub struct MysqlUrl {
    url: Url,
    pub(crate) query_params: MysqlUrlQueryParams,
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

    pub(crate) fn statement_cache_size(&self) -> usize {
        self.query_params.statement_cache_size
    }

    fn parse_query_params(url: &Url) -> Result<MysqlUrlQueryParams, Error> {
        #[cfg(feature = "mysql-connector")]
        let mut ssl_opts = {
            let mut ssl_opts = mysql_async::SslOpts::default();
            ssl_opts = ssl_opts.with_danger_accept_invalid_certs(true);
            ssl_opts
        };

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

                    #[cfg(feature = "mysql-connector")]
                    {
                        ssl_opts = ssl_opts.with_root_cert_path(Some(Path::new(&*v).to_path_buf()));
                    }
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
                            #[cfg(feature = "mysql-connector")]
                            {
                                ssl_opts = ssl_opts.with_danger_accept_invalid_certs(false);
                            }
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

        // Wrapping this in a block, as attributes on expressions are still experimental
        // See: https://github.com/rust-lang/rust/issues/15701
        #[cfg(feature = "mysql-connector")]
        {
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
        }

        Ok(MysqlUrlQueryParams {
            #[cfg(feature = "mysql-connector")]
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
}

#[derive(Debug, Clone)]
pub(crate) struct MysqlUrlQueryParams {
    pub(crate) connection_limit: Option<usize>,
    pub(crate) use_ssl: bool,
    pub(crate) socket: Option<String>,
    pub(crate) socket_timeout: Option<Duration>,
    pub(crate) connect_timeout: Option<Duration>,
    pub(crate) pool_timeout: Option<Duration>,
    pub(crate) max_connection_lifetime: Option<Duration>,
    pub(crate) max_idle_connection_lifetime: Option<Duration>,
    pub(crate) prefer_socket: Option<bool>,
    pub(crate) statement_cache_size: usize,

    #[cfg(feature = "mysql-connector")]
    pub(crate) ssl_opts: mysql_async::SslOpts,
}
