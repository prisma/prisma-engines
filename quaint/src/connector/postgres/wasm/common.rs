use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    time::Duration,
};

use percent_encoding::percent_decode;
use url::{Host, Url};

use crate::error::{Error, ErrorKind};

#[cfg(feature = "postgresql-connector")]
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
        #[cfg(feature = "postgresql-connector")]
        let mut ssl_mode = SslMode::Prefer;
        #[cfg(feature = "postgresql-connector")]
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
                #[cfg(feature = "postgresql-connector")]
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
                #[cfg(feature = "postgresql-connector")]
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
            #[cfg(feature = "postgresql-connector")]
            channel_binding,
            #[cfg(feature = "postgresql-connector")]
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

    #[cfg(feature = "postgresql-connector")]
    pub(crate) channel_binding: ChannelBinding,

    #[cfg(feature = "postgresql-connector")]
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

/// Sorted list of CockroachDB's reserved keywords.
/// Taken from https://www.cockroachlabs.com/docs/stable/keywords-and-identifiers.html#keywords
const RESERVED_KEYWORDS: [&str; 79] = [
    "all",
    "analyse",
    "analyze",
    "and",
    "any",
    "array",
    "as",
    "asc",
    "asymmetric",
    "both",
    "case",
    "cast",
    "check",
    "collate",
    "column",
    "concurrently",
    "constraint",
    "create",
    "current_catalog",
    "current_date",
    "current_role",
    "current_schema",
    "current_time",
    "current_timestamp",
    "current_user",
    "default",
    "deferrable",
    "desc",
    "distinct",
    "do",
    "else",
    "end",
    "except",
    "false",
    "fetch",
    "for",
    "foreign",
    "from",
    "grant",
    "group",
    "having",
    "in",
    "initially",
    "intersect",
    "into",
    "lateral",
    "leading",
    "limit",
    "localtime",
    "localtimestamp",
    "not",
    "null",
    "offset",
    "on",
    "only",
    "or",
    "order",
    "placing",
    "primary",
    "references",
    "returning",
    "select",
    "session_user",
    "some",
    "symmetric",
    "table",
    "then",
    "to",
    "trailing",
    "true",
    "union",
    "unique",
    "user",
    "using",
    "variadic",
    "when",
    "where",
    "window",
    "with",
];

/// Sorted list of CockroachDB's reserved type function names.
/// Taken from https://www.cockroachlabs.com/docs/stable/keywords-and-identifiers.html#keywords
const RESERVED_TYPE_FUNCTION_NAMES: [&str; 18] = [
    "authorization",
    "collation",
    "cross",
    "full",
    "ilike",
    "inner",
    "is",
    "isnull",
    "join",
    "left",
    "like",
    "natural",
    "none",
    "notnull",
    "outer",
    "overlaps",
    "right",
    "similar",
];

/// Returns true if a Postgres identifier is considered "safe".
///
/// In this context, "safe" means that the value of an identifier would be the same quoted and unquoted or that it's not part of reserved keywords. In other words, that it does _not_ need to be quoted.
///
/// Spec can be found here: https://www.postgresql.org/docs/current/sql-syntax-lexical.html#SQL-SYNTAX-IDENTIFIERS
/// or here: https://www.cockroachlabs.com/docs/stable/keywords-and-identifiers.html#rules-for-identifiers
fn is_safe_identifier(ident: &str) -> bool {
    if ident.is_empty() {
        return false;
    }

    // 1. Not equal any SQL keyword unless the keyword is accepted by the element's syntax. For example, name accepts Unreserved or Column Name keywords.
    if RESERVED_KEYWORDS.binary_search(&ident).is_ok() || RESERVED_TYPE_FUNCTION_NAMES.binary_search(&ident).is_ok() {
        return false;
    }

    let mut chars = ident.chars();

    let first = chars.next().unwrap();

    // 2. SQL identifiers must begin with a letter (a-z, but also letters with diacritical marks and non-Latin letters) or an underscore (_).
    if (!first.is_alphabetic() || !first.is_lowercase()) && first != '_' {
        return false;
    }

    for c in chars {
        // 3. Subsequent characters in an identifier can be letters, underscores, digits (0-9), or dollar signs ($).
        if (!c.is_alphabetic() || !c.is_lowercase()) && c != '_' && !c.is_ascii_digit() && c != '$' {
            return false;
        }
    }

    true
}
