#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    time::Duration,
};

use percent_encoding::percent_decode;
use url::{Host, Url};

use crate::error::{Error, ErrorKind};

#[cfg(feature = "postgresql-native")]
use tokio_postgres::config::{ChannelBinding, SslMode};

#[derive(Clone)]
pub(crate) struct Hidden<T>(pub(crate) T);

impl<T> Debug for Hidden<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<HIDDEN>")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SslAcceptMode {
    Strict,
    AcceptInvalidCerts,
}

#[derive(Debug, Clone)]
pub struct SslParams {
    pub(crate) certificate_file: Option<String>,
    pub(crate) identity_file: Option<String>,
    pub(crate) identity_password: Hidden<Option<String>>,
    pub(crate) ssl_accept_mode: SslAcceptMode,
}

#[derive(Debug, Clone, Copy)]
pub enum PostgresFlavour {
    Postgres,
    Cockroach,
    Unknown,
}

impl PostgresFlavour {
    /// Returns `true` if the postgres flavour is [`Postgres`].
    ///
    /// [`Postgres`]: PostgresFlavour::Postgres
    pub(crate) fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres)
    }

    /// Returns `true` if the postgres flavour is [`Cockroach`].
    ///
    /// [`Cockroach`]: PostgresFlavour::Cockroach
    pub(crate) fn is_cockroach(&self) -> bool {
        matches!(self, Self::Cockroach)
    }

    /// Returns `true` if the postgres flavour is [`Unknown`].
    ///
    /// [`Unknown`]: PostgresFlavour::Unknown
    pub(crate) fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// Wraps a connection url and exposes the parsing logic used by Quaint,
/// including default values.
#[derive(Debug, Clone)]
pub struct PostgresUrl {
    pub(crate) url: Url,
    pub(crate) query_params: PostgresUrlQueryParams,
    pub(crate) flavour: PostgresFlavour,
}

pub(crate) const DEFAULT_SCHEMA: &str = "public";

impl PostgresUrl {
    /// Parse `Url` to `PostgresUrl`. Returns error for mistyped connection
    /// parameters.
    pub fn new(url: Url) -> Result<Self, Error> {
        let query_params = Self::parse_query_params(&url)?;

        Ok(Self {
            url,
            query_params,
            flavour: PostgresFlavour::Unknown,
        })
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

    /// The database host. Taken first from the `host` query parameter, then
    /// from the `host` part of the URL. For socket connections, the query
    /// parameter must be used.
    ///
    /// If none of them are set, defaults to `localhost`.
    pub fn host(&self) -> &str {
        match (self.query_params.host.as_ref(), self.url.host_str(), self.url.host()) {
            (Some(host), _, _) => host.as_str(),
            (None, Some(""), _) => "localhost",
            (None, None, _) => "localhost",
            (None, Some(host), Some(Host::Ipv6(_))) => {
                // The `url` crate may return an IPv6 address in brackets, which must be stripped.
                if host.starts_with('[') && host.ends_with(']') {
                    &host[1..host.len() - 1]
                } else {
                    host
                }
            }
            (None, Some(host), _) => host,
        }
    }

    /// Name of the database connected. Defaults to `postgres`.
    pub fn dbname(&self) -> &str {
        match self.url.path_segments() {
            Some(mut segments) => segments.next().unwrap_or("postgres"),
            None => "postgres",
        }
    }

    /// The percent-decoded database password.
    pub fn password(&self) -> Cow<str> {
        match self
            .url
            .password()
            .and_then(|pw| percent_decode(pw.as_bytes()).decode_utf8().ok())
        {
            Some(password) => password,
            None => self.url.password().unwrap_or("").into(),
        }
    }

    /// The database port, defaults to `5432`.
    pub fn port(&self) -> u16 {
        self.url.port().unwrap_or(5432)
    }

    /// The database schema, defaults to `public`.
    pub fn schema(&self) -> &str {
        self.query_params.schema.as_deref().unwrap_or(DEFAULT_SCHEMA)
    }

    /// Whether the pgbouncer mode is enabled.
    pub fn pg_bouncer(&self) -> bool {
        self.query_params.pg_bouncer
    }

    /// The connection timeout.
    pub fn connect_timeout(&self) -> Option<Duration> {
        self.query_params.connect_timeout
    }

    /// Pool check_out timeout
    pub fn pool_timeout(&self) -> Option<Duration> {
        self.query_params.pool_timeout
    }

    /// The socket timeout
    pub fn socket_timeout(&self) -> Option<Duration> {
        self.query_params.socket_timeout
    }

    /// The maximum connection lifetime
    pub fn max_connection_lifetime(&self) -> Option<Duration> {
        self.query_params.max_connection_lifetime
    }

    /// The maximum idle connection lifetime
    pub fn max_idle_connection_lifetime(&self) -> Option<Duration> {
        self.query_params.max_idle_connection_lifetime
    }

    /// The custom application name
    pub fn application_name(&self) -> Option<&str> {
        self.query_params.application_name.as_deref()
    }

    pub(crate) fn options(&self) -> Option<&str> {
        self.query_params.options.as_deref()
    }

    /// Sets whether the URL points to a Postgres, Cockroach or Unknown database.
    /// This is used to avoid a network roundtrip at connection to set the search path.
    ///
    /// The different behaviours are:
    /// - Postgres: Always avoid a network roundtrip by setting the search path through client connection parameters.
    /// - Cockroach: Avoid a network roundtrip if the schema name is deemed "safe" (i.e. no escape quoting required). Otherwise, set the search path through a database query.
    /// - Unknown: Always add a network roundtrip by setting the search path through a database query.
    pub fn set_flavour(&mut self, flavour: PostgresFlavour) {
        self.flavour = flavour;
    }

    fn parse_query_params(url: &Url) -> Result<PostgresUrlQueryParams, Error> {
        #[cfg(feature = "postgresql-native")]
        let mut ssl_mode = SslMode::Prefer;
        #[cfg(feature = "postgresql-native")]
        let mut channel_binding = ChannelBinding::Prefer;

        let mut connection_limit = None;
        let mut schema = None;
        let mut certificate_file = None;
        let mut identity_file = None;
        let mut identity_password = None;
        let mut ssl_accept_mode = SslAcceptMode::AcceptInvalidCerts;
        let mut host = None;
        let mut application_name = None;
        let mut socket_timeout = None;
        let mut connect_timeout = Some(Duration::from_secs(5));
        let mut pool_timeout = Some(Duration::from_secs(10));
        let mut pg_bouncer = false;
        let mut statement_cache_size = 100;
        let mut max_connection_lifetime = None;
        let mut max_idle_connection_lifetime = Some(Duration::from_secs(300));
        let mut options = None;

        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "pgbouncer" => {
                    pg_bouncer = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                }
                #[cfg(feature = "postgresql-native")]
                "sslmode" => {
                    match v.as_ref() {
                        "disable" => ssl_mode = SslMode::Disable,
                        "prefer" => ssl_mode = SslMode::Prefer,
                        "require" => ssl_mode = SslMode::Require,
                        _ => {
                            tracing::debug!(message = "Unsupported SSL mode, defaulting to `prefer`", mode = &*v);
                        }
                    };
                }
                "sslcert" => {
                    certificate_file = Some(v.to_string());
                }
                "sslidentity" => {
                    identity_file = Some(v.to_string());
                }
                "sslpassword" => {
                    identity_password = Some(v.to_string());
                }
                "statement_cache_size" => {
                    statement_cache_size = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                }
                "sslaccept" => {
                    match v.as_ref() {
                        "strict" => {
                            ssl_accept_mode = SslAcceptMode::Strict;
                        }
                        "accept_invalid_certs" => {
                            ssl_accept_mode = SslAcceptMode::AcceptInvalidCerts;
                        }
                        _ => {
                            tracing::debug!(
                                message = "Unsupported SSL accept mode, defaulting to `strict`",
                                mode = &*v
                            );

                            ssl_accept_mode = SslAcceptMode::Strict;
                        }
                    };
                }
                "schema" => {
                    schema = Some(v.to_string());
                }
                "connection_limit" => {
                    let as_int: usize = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                    connection_limit = Some(as_int);
                }
                "host" => {
                    host = Some(v.to_string());
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

                    if as_int == 0 {
                        connect_timeout = None;
                    } else {
                        connect_timeout = Some(Duration::from_secs(as_int));
                    }
                }
                "pool_timeout" => {
                    let as_int = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    if as_int == 0 {
                        pool_timeout = None;
                    } else {
                        pool_timeout = Some(Duration::from_secs(as_int));
                    }
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
                "application_name" => {
                    application_name = Some(v.to_string());
                }
                #[cfg(feature = "postgresql-native")]
                "channel_binding" => {
                    match v.as_ref() {
                        "disable" => channel_binding = ChannelBinding::Disable,
                        "prefer" => channel_binding = ChannelBinding::Prefer,
                        "require" => channel_binding = ChannelBinding::Require,
                        _ => {
                            tracing::debug!(
                                message = "Unsupported Channel Binding {channel_binding}, defaulting to `prefer`",
                                channel_binding = &*v
                            );
                        }
                    };
                }
                "options" => {
                    options = Some(v.to_string());
                }
                _ => {
                    tracing::trace!(message = "Discarding connection string param", param = &*k);
                }
            };
        }

        Ok(PostgresUrlQueryParams {
            ssl_params: SslParams {
                certificate_file,
                identity_file,
                ssl_accept_mode,
                identity_password: Hidden(identity_password),
            },
            connection_limit,
            schema,
            host,
            connect_timeout,
            pool_timeout,
            socket_timeout,
            pg_bouncer,
            statement_cache_size,
            max_connection_lifetime,
            max_idle_connection_lifetime,
            application_name,
            options,
            #[cfg(feature = "postgresql-native")]
            channel_binding,
            #[cfg(feature = "postgresql-native")]
            ssl_mode,
        })
    }

    pub(crate) fn ssl_params(&self) -> &SslParams {
        &self.query_params.ssl_params
    }

    #[cfg(feature = "pooled")]
    pub(crate) fn connection_limit(&self) -> Option<usize> {
        self.query_params.connection_limit
    }

    pub fn flavour(&self) -> PostgresFlavour {
        self.flavour
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresUrlQueryParams {
    pub(crate) ssl_params: SslParams,
    pub(crate) connection_limit: Option<usize>,
    pub(crate) schema: Option<String>,
    pub(crate) pg_bouncer: bool,
    pub(crate) host: Option<String>,
    pub(crate) socket_timeout: Option<Duration>,
    pub(crate) connect_timeout: Option<Duration>,
    pub(crate) pool_timeout: Option<Duration>,
    pub(crate) statement_cache_size: usize,
    pub(crate) max_connection_lifetime: Option<Duration>,
    pub(crate) max_idle_connection_lifetime: Option<Duration>,
    pub(crate) application_name: Option<String>,
    pub(crate) options: Option<String>,

    #[cfg(feature = "postgresql-native")]
    pub(crate) channel_binding: ChannelBinding,

    #[cfg(feature = "postgresql-native")]
    pub(crate) ssl_mode: SslMode,
}

// A SearchPath connection parameter (Display-impl) for connection initialization.
struct CockroachSearchPath<'a>(&'a str);

impl Display for CockroachSearchPath<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

// A SearchPath connection parameter (Display-impl) for connection initialization.
struct PostgresSearchPath<'a>(&'a str);

impl Display for PostgresSearchPath<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        f.write_str(self.0)?;
        f.write_str("\"")?;

        Ok(())
    }
}

// A SetSearchPath statement (Display-impl) for connection initialization.
struct SetSearchPath<'a>(Option<&'a str>);

impl Display for SetSearchPath<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = self.0 {
            f.write_str("SET search_path = \"")?;
            f.write_str(schema)?;
            f.write_str("\";\n")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Value;
    pub(crate) use crate::connector::postgres::url::PostgresFlavour;
    use crate::tests::test_api::postgres::CONN_STR;
    use crate::{connector::Queryable, error::*, single::Quaint};
    use url::Url;

    #[test]
    fn should_parse_socket_url() {
        let url = PostgresUrl::new(Url::parse("postgresql:///dbname?host=/var/run/psql.sock").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!("/var/run/psql.sock", url.host());
    }

    #[test]
    fn should_parse_escaped_url() {
        let url = PostgresUrl::new(Url::parse("postgresql:///dbname?host=%2Fvar%2Frun%2Fpostgresql").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!("/var/run/postgresql", url.host());
    }

    #[test]
    fn should_allow_changing_of_cache_size() {
        let url =
            PostgresUrl::new(Url::parse("postgresql:///localhost:5432/foo?statement_cache_size=420").unwrap()).unwrap();
        assert_eq!(420, url.cache().capacity());
    }

    #[test]
    fn should_have_default_cache_size() {
        let url = PostgresUrl::new(Url::parse("postgresql:///localhost:5432/foo").unwrap()).unwrap();
        assert_eq!(100, url.cache().capacity());
    }

    #[test]
    fn should_have_application_name() {
        let url =
            PostgresUrl::new(Url::parse("postgresql:///localhost:5432/foo?application_name=test").unwrap()).unwrap();
        assert_eq!(Some("test"), url.application_name());
    }

    #[test]
    fn should_have_channel_binding() {
        let url =
            PostgresUrl::new(Url::parse("postgresql:///localhost:5432/foo?channel_binding=require").unwrap()).unwrap();
        assert_eq!(ChannelBinding::Require, url.channel_binding());
    }

    #[test]
    fn should_have_default_channel_binding() {
        let url =
            PostgresUrl::new(Url::parse("postgresql:///localhost:5432/foo?channel_binding=invalid").unwrap()).unwrap();
        assert_eq!(ChannelBinding::Prefer, url.channel_binding());

        let url = PostgresUrl::new(Url::parse("postgresql:///localhost:5432/foo").unwrap()).unwrap();
        assert_eq!(ChannelBinding::Prefer, url.channel_binding());
    }

    #[test]
    fn should_not_enable_caching_with_pgbouncer() {
        let url = PostgresUrl::new(Url::parse("postgresql:///localhost:5432/foo?pgbouncer=true").unwrap()).unwrap();
        assert_eq!(0, url.cache().capacity());
    }

    #[test]
    fn should_parse_default_host() {
        let url = PostgresUrl::new(Url::parse("postgresql:///dbname").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!("localhost", url.host());
    }

    #[test]
    fn should_parse_ipv6_host() {
        let url = PostgresUrl::new(Url::parse("postgresql://[2001:db8:1234::ffff]:5432/dbname").unwrap()).unwrap();
        assert_eq!("2001:db8:1234::ffff", url.host());
    }

    #[test]
    fn should_handle_options_field() {
        let url = PostgresUrl::new(Url::parse("postgresql:///localhost:5432?options=--cluster%3Dmy_cluster").unwrap())
            .unwrap();

        assert_eq!("--cluster=my_cluster", url.options().unwrap());
    }

    #[tokio::test]
    async fn should_map_nonexisting_database_error() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.set_path("/this_does_not_exist");

        let res = Quaint::new(url.as_str()).await;

        assert!(res.is_err());

        match res {
            Ok(_) => unreachable!(),
            Err(e) => match e.kind() {
                ErrorKind::DatabaseDoesNotExist { db_name } => {
                    assert_eq!(Some("3D000"), e.original_code());
                    assert_eq!(
                        Some("database \"this_does_not_exist\" does not exist"),
                        e.original_message()
                    );
                    assert_eq!(&Name::available("this_does_not_exist"), db_name)
                }
                kind => panic!("Expected `DatabaseDoesNotExist`, got {:?}", kind),
            },
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

    #[tokio::test]
    async fn should_map_tls_errors() {
        let mut url = Url::parse(&CONN_STR).expect("parsing url");
        url.set_query(Some("sslmode=require&sslaccept=strict"));

        let res = Quaint::new(url.as_str()).await;

        assert!(res.is_err());

        match res {
            Ok(_) => unreachable!(),
            Err(e) => match e.kind() {
                ErrorKind::TlsError { .. } => (),
                other => panic!("{:#?}", other),
            },
        }
    }

    #[tokio::test]
    async fn should_map_incorrect_parameters_error() {
        let url = Url::parse(&CONN_STR).unwrap();
        let conn = Quaint::new(url.as_str()).await.unwrap();

        let res = conn.query_raw("SELECT $1", &[Value::int32(1), Value::int32(2)]).await;

        assert!(res.is_err());

        match res {
            Ok(_) => unreachable!(),
            Err(e) => match e.kind() {
                ErrorKind::IncorrectNumberOfParameters { expected, actual } => {
                    assert_eq!(1, *expected);
                    assert_eq!(2, *actual);
                }
                other => panic!("{:#?}", other),
            },
        }
    }

    #[test]
    fn search_path_pgbouncer_should_be_set_with_query() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.query_pairs_mut().append_pair("schema", "hello");
        url.query_pairs_mut().append_pair("pgbouncer", "true");

        let mut pg_url = PostgresUrl::new(url).unwrap();
        pg_url.set_flavour(PostgresFlavour::Postgres);

        let config = pg_url.to_config();

        // PGBouncer does not support the `search_path` connection parameter.
        // When `pgbouncer=true`, config.search_path should be None,
        // And the `search_path` should be set via a db query after connection.
        assert_eq!(config.get_search_path(), None);
    }

    #[test]
    fn search_path_pg_should_be_set_with_param() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.query_pairs_mut().append_pair("schema", "hello");

        let mut pg_url = PostgresUrl::new(url).unwrap();
        pg_url.set_flavour(PostgresFlavour::Postgres);

        let config = pg_url.to_config();

        // Postgres supports setting the search_path via a connection parameter.
        assert_eq!(config.get_search_path(), Some(&"\"hello\"".to_owned()));
    }

    #[test]
    fn search_path_crdb_safe_ident_should_be_set_with_param() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.query_pairs_mut().append_pair("schema", "hello");

        let mut pg_url = PostgresUrl::new(url).unwrap();
        pg_url.set_flavour(PostgresFlavour::Cockroach);

        let config = pg_url.to_config();

        // CRDB supports setting the search_path via a connection parameter if the identifier is safe.
        assert_eq!(config.get_search_path(), Some(&"hello".to_owned()));
    }

    #[test]
    fn search_path_crdb_unsafe_ident_should_be_set_with_query() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.query_pairs_mut().append_pair("schema", "HeLLo");

        let mut pg_url = PostgresUrl::new(url).unwrap();
        pg_url.set_flavour(PostgresFlavour::Cockroach);

        let config = pg_url.to_config();

        // CRDB does NOT support setting the search_path via a connection parameter if the identifier is unsafe.
        assert_eq!(config.get_search_path(), None);
    }
}
