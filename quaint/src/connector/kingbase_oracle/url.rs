use crate::error::{Error, ErrorKind};
use std::{fmt, time::Duration};
use url::Url;

#[derive(Clone)]
pub(crate) struct Hidden<T>(pub(crate) T);

impl<T> fmt::Debug for Hidden<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<HIDDEN>")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SslAcceptMode {
    Strict,
    AcceptInvalidCerts,
}

#[derive(Debug, Clone)]
pub(crate) struct SslParams {
    pub(crate) certificate_file: Option<String>,
    pub(crate) identity_file: Option<String>,
    pub(crate) identity_password: Hidden<Option<String>>,
    pub(crate) ssl_accept_mode: SslAcceptMode,
}

/// A KingbaseES connection URL for the Oracle-compatible provider.
#[derive(Clone)]
pub struct KingbaseOracleUrl {
    pub(crate) url: Url,
    pub(crate) query_params: KingbaseOracleUrlQueryParams,
}

impl fmt::Debug for KingbaseOracleUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KingbaseOracleUrl")
            .field("scheme", &self.url.scheme())
            .field("host", &self.host())
            .field("port", &self.port())
            .field("database", &self.dbname())
            .field("query_params", &self.query_params)
            .finish()
    }
}

impl KingbaseOracleUrl {
    pub fn new(url: Url) -> crate::Result<Self> {
        if url.scheme() != "kingbase-oracle" {
            let kind = ErrorKind::DatabaseUrlIsInvalid(format!(
                "{} is not a supported Kingbase Oracle database URL scheme.",
                url.scheme()
            ));

            return Err(Error::builder(kind).build());
        }

        let query_params = KingbaseOracleUrlQueryParams::parse(&url)?;

        Ok(Self { url, query_params })
    }

    pub(crate) fn connect_timeout(&self) -> Option<Duration> {
        self.query_params.connect_timeout
    }

    pub fn dbname(&self) -> Option<&str> {
        self.url.path().strip_prefix('/').filter(|name| !name.is_empty())
    }

    pub fn schema(&self) -> Option<&str> {
        self.query_params.schema.as_deref()
    }

    pub fn host(&self) -> &str {
        self.url.host_str().unwrap_or("localhost")
    }

    pub(crate) fn username(&self) -> &str {
        self.url.username()
    }

    pub fn port(&self) -> u16 {
        self.url.port().unwrap_or(super::DEFAULT_KINGBASE_ORACLE_PORT)
    }

    pub(crate) fn socket_timeout(&self) -> Option<Duration> {
        self.query_params.socket_timeout
    }

    pub(crate) fn connection_limit(&self) -> Option<usize> {
        self.query_params.connection_limit
    }

    pub(crate) fn pool_timeout(&self) -> Option<Duration> {
        self.query_params.pool_timeout
    }

    pub(crate) fn max_connection_lifetime(&self) -> Option<Duration> {
        self.query_params.max_connection_lifetime
    }

    pub(crate) fn max_idle_connection_lifetime(&self) -> Option<Duration> {
        self.query_params.max_idle_connection_lifetime
    }

    pub(crate) fn pg_bouncer(&self) -> bool {
        self.query_params.pg_bouncer
    }

    pub(crate) fn ssl_params(&self) -> &SslParams {
        &self.query_params.ssl_params
    }
}

#[derive(Debug, Clone)]
pub(crate) struct KingbaseOracleUrlQueryParams {
    pub(crate) ssl_params: SslParams,
    pub(crate) schema: Option<String>,
    pub(crate) pg_bouncer: bool,
    pub(crate) connect_timeout: Option<Duration>,
    pub(crate) socket_timeout: Option<Duration>,
    pub(crate) connection_limit: Option<usize>,
    pub(crate) pool_timeout: Option<Duration>,
    pub(crate) max_connection_lifetime: Option<Duration>,
    pub(crate) max_idle_connection_lifetime: Option<Duration>,
    pub(crate) user: Option<String>,
    pub(crate) password: Hidden<Option<String>>,
    pub(crate) dbname: Option<String>,
    pub(crate) options: Option<String>,
    pub(crate) application_name: Option<String>,
    pub(crate) ssl_mode: Option<String>,
    pub(crate) ssl_negotiation: Option<String>,
    pub(crate) hosts: Vec<String>,
    pub(crate) hostaddrs: Vec<std::net::IpAddr>,
    pub(crate) ports: Vec<u16>,
    pub(crate) tcp_user_timeout: Option<Duration>,
    pub(crate) keepalives: Option<bool>,
    pub(crate) keepalives_idle: Option<Duration>,
    pub(crate) keepalives_interval: Option<Duration>,
    pub(crate) keepalives_retries: Option<u32>,
    pub(crate) target_session_attrs: Option<String>,
    pub(crate) channel_binding: Option<String>,
    pub(crate) load_balance_hosts: Option<String>,
    pub(crate) driver_connect_timeout: Option<Duration>,
}

impl KingbaseOracleUrlQueryParams {
    fn parse(url: &Url) -> crate::Result<Self> {
        let mut user = None;
        let mut password = None;
        let mut dbname = None;
        let mut options = None;
        let mut application_name = None;
        let mut ssl_mode = None;
        let mut ssl_negotiation = None;
        let mut hosts = Vec::new();
        let mut hostaddrs = Vec::new();
        let mut ports = Vec::new();
        let mut tcp_user_timeout = None;
        let mut keepalives = None;
        let mut keepalives_idle = None;
        let mut keepalives_interval = None;
        let mut keepalives_retries = None;
        let mut target_session_attrs = None;
        let mut channel_binding = None;
        let mut load_balance_hosts = None;
        let mut driver_connect_timeout = None;
        let mut schema = None;
        let mut certificate_file = None;
        let mut identity_file = None;
        let mut identity_password = None;
        let mut ssl_accept_mode = SslAcceptMode::Strict;
        let mut pg_bouncer = false;
        let mut connect_timeout = None;
        let mut socket_timeout = None;
        let mut connection_limit = None;
        let mut pool_timeout = None;
        let mut max_connection_lifetime = None;
        let mut max_idle_connection_lifetime = None;

        for (key, value) in url.query_pairs() {
            // The previous URL round-trip encoded spaces as '+', which the native
            // driver treated literally. Preserve those values during this refactor.
            let driver_value = value.replace(' ', "+");
            match key.as_ref() {
                "schema" => schema = Some(value.into_owned()),
                "sslcert" => certificate_file = Some(value.into_owned()),
                "sslidentity" => identity_file = Some(value.into_owned()),
                "sslpassword" => identity_password = Some(value.into_owned()),
                "sslaccept" => {
                    ssl_accept_mode = match value.as_ref() {
                        "strict" => SslAcceptMode::Strict,
                        "accept_invalid_certs" => SslAcceptMode::AcceptInvalidCerts,
                        _ => {
                            tracing::debug!(
                                message = "Unsupported SSL accept mode, defaulting to `strict`",
                                mode = &*value
                            );

                            SslAcceptMode::Strict
                        }
                    };
                }
                "pgbouncer" => {
                    pg_bouncer = value
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                }
                "connect_timeout" => {
                    connect_timeout = parse_optional_timeout(&value)?;
                    if let Some(timeout) = parse_positive_timeout(&value)? {
                        driver_connect_timeout = Some(timeout);
                    }
                }
                "socket_timeout" => socket_timeout = parse_optional_timeout(&value)?,
                "connection_limit" => {
                    connection_limit = Some(
                        value
                            .parse()
                            .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?,
                    );
                }
                "pool_timeout" => pool_timeout = parse_optional_timeout(&value)?,
                "max_connection_lifetime" => max_connection_lifetime = parse_optional_timeout(&value)?,
                "max_idle_connection_lifetime" => max_idle_connection_lifetime = parse_optional_timeout(&value)?,
                "user" => user = Some(driver_value.clone()),
                "password" => password = Some(driver_value.clone()),
                "dbname" => dbname = Some(driver_value.clone()),
                "options" => options = Some(driver_value.clone()),
                "application_name" => application_name = Some(driver_value.clone()),
                "sslmode" => ssl_mode = Some(parse_choice(&driver_value, &["disable", "prefer", "require"])?),
                "sslnegotiation" => ssl_negotiation = Some(parse_choice(&driver_value, &["postgres", "direct"])?),
                "host" => hosts.push(driver_value.clone()),
                "hostaddr" => {
                    for address in driver_value.split(',') {
                        hostaddrs.push(parse_connection_value(address)?);
                    }
                }
                "port" => {
                    for port in driver_value.split(',') {
                        // Preserve the driver's handling of an empty query-string port.
                        ports.push(if port.is_empty() {
                            5432
                        } else {
                            parse_connection_value(port)?
                        });
                    }
                }
                "tcp_user_timeout" => {
                    if let Some(timeout) = parse_positive_timeout(&driver_value)? {
                        tcp_user_timeout = Some(timeout);
                    }
                }
                "keepalives" => keepalives = Some(parse_connection_value::<u64>(&driver_value)? != 0),
                "keepalives_idle" => {
                    if let Some(timeout) = parse_positive_timeout(&driver_value)? {
                        keepalives_idle = Some(timeout);
                    }
                }
                "keepalives_interval" => {
                    if let Some(timeout) = parse_positive_timeout(&driver_value)? {
                        keepalives_interval = Some(timeout);
                    }
                }
                "keepalives_retries" => keepalives_retries = Some(parse_connection_value(&driver_value)?),
                "target_session_attrs" => {
                    target_session_attrs = Some(parse_choice(&driver_value, &["any", "read-write", "read-only"])?);
                }
                "channel_binding" => {
                    channel_binding = Some(parse_choice(&driver_value, &["disable", "prefer", "require"])?)
                }
                "load_balance_hosts" => load_balance_hosts = Some(parse_choice(&driver_value, &["disable", "random"])?),
                "statement_cache_size" => (),
                key => {
                    return Err(
                        Error::builder(ErrorKind::DatabaseUrlIsInvalid(format!("unknown option `{key}`"))).build(),
                    );
                }
            }
        }

        Ok(Self {
            user,
            password: Hidden(password),
            dbname,
            options,
            application_name,
            ssl_mode,
            ssl_negotiation,
            hosts,
            hostaddrs,
            ports,
            tcp_user_timeout,
            keepalives,
            keepalives_idle,
            keepalives_interval,
            keepalives_retries,
            target_session_attrs,
            channel_binding,
            load_balance_hosts,
            driver_connect_timeout,
            ssl_params: SslParams {
                certificate_file,
                identity_file,
                identity_password: Hidden(identity_password),
                ssl_accept_mode,
            },
            schema,
            pg_bouncer,
            connect_timeout,
            socket_timeout,
            connection_limit,
            pool_timeout,
            max_connection_lifetime,
            max_idle_connection_lifetime,
        })
    }
}

fn parse_connection_value<T: std::str::FromStr>(value: &str) -> crate::Result<T> {
    value.parse().map_err(|_| {
        Error::builder(ErrorKind::DatabaseUrlIsInvalid(
            "invalid connection option value".into(),
        ))
        .build()
    })
}

fn parse_choice(value: &str, choices: &[&str]) -> crate::Result<String> {
    if choices.contains(&value) {
        Ok(value.to_owned())
    } else {
        Err(Error::builder(ErrorKind::DatabaseUrlIsInvalid(
            "invalid connection option value".into(),
        ))
        .build())
    }
}

fn parse_positive_timeout(value: &str) -> crate::Result<Option<Duration>> {
    let seconds = parse_connection_value::<i64>(value)?;
    Ok((seconds > 0).then(|| Duration::from_secs(seconds as u64)))
}

fn parse_optional_timeout(value: &str) -> crate::Result<Option<Duration>> {
    let seconds = value
        .parse::<u64>()
        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

    Ok((seconds != 0).then(|| Duration::from_secs(seconds)))
}

#[cfg(test)]
mod tests {
    use super::{KingbaseOracleUrl, SslAcceptMode};
    use url::Url;

    #[cfg(feature = "kingbase-oracle-native")]
    #[test]
    fn configures_kingbase_oracle_driver_settings() {
        let url = KingbaseOracleUrl::new(
            Url::parse("kingbase-oracle://user:password@localhost:54325/app?schema=tenant&pgbouncer=true").unwrap(),
        )
        .unwrap();

        let config = url.to_config().unwrap();

        assert!(config.get_pgbouncer_mode());
        assert_eq!(config.get_search_path().map(String::as_str), Some("\"tenant\""));
    }

    #[test]
    fn rejects_non_oracle_kingbase_schemes() {
        for scheme in ["kingbase", "kingbase-mysql", "postgresql"] {
            let error = KingbaseOracleUrl::new(Url::parse(&format!("{scheme}://localhost/app")).unwrap()).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("not a supported Kingbase Oracle database URL scheme")
            );
        }
    }

    #[cfg(feature = "kingbase-oracle-native")]
    #[test]
    fn uses_kingbase_default_port_when_the_url_omits_one() {
        let url = KingbaseOracleUrl::new(Url::parse("kingbase-oracle://localhost/app").unwrap()).unwrap();

        assert_eq!(url.port(), 54321);
        assert_eq!(url.to_config().unwrap().get_ports(), &[54321]);
    }

    #[test]
    fn defaults_to_strict_sslaccept_mode() {
        let url =
            KingbaseOracleUrl::new(Url::parse("kingbase-oracle://localhost/app?sslmode=require").unwrap()).unwrap();

        assert_eq!(url.ssl_params().ssl_accept_mode, SslAcceptMode::Strict);
    }

    #[test]
    fn parses_explicit_accept_invalid_certs_sslaccept_mode() {
        let url = KingbaseOracleUrl::new(
            Url::parse("kingbase-oracle://localhost/app?sslmode=require&sslaccept=accept_invalid_certs").unwrap(),
        )
        .unwrap();

        assert_eq!(url.ssl_params().ssl_accept_mode, SslAcceptMode::AcceptInvalidCerts);
    }

    #[test]
    fn debug_output_hides_connection_credentials() {
        let url = KingbaseOracleUrl::new(
            Url::parse(
                "kingbase-oracle://user:database-secret@localhost/app?sslidentity=client.p12&sslpassword=identity-secret",
            )
            .unwrap(),
        )
        .unwrap();

        let debug = format!("{url:?}");

        assert!(!debug.contains("database-secret"));
        assert!(!debug.contains("identity-secret"));
        assert!(debug.contains("<HIDDEN>"));
    }
}

#[cfg(all(test, feature = "kingbase-oracle-native"))]
mod config_tests {
    use super::*;
    use kingbase_tokio_postgres::Config;

    // Keep the previous URL-based path as a regression oracle while the production
    // connector now constructs Config explicitly in native/mod.rs.
    fn previous_config(url: &Url) -> crate::Result<Config> {
        let mut driver_url = url.clone();
        driver_url.set_scheme("kingbase").map_err(|_| {
            Error::builder(ErrorKind::DatabaseUrlIsInvalid(
                "invalid Kingbase Oracle URL scheme".into(),
            ))
            .build()
        })?;

        if driver_url.port().is_none() {
            driver_url
                .set_port(Some(super::super::DEFAULT_KINGBASE_ORACLE_PORT))
                .map_err(|_| {
                    Error::builder(ErrorKind::DatabaseUrlIsInvalid(
                        "invalid Kingbase Oracle URL port".into(),
                    ))
                    .build()
                })?;
        }

        let driver_params = url
            .query_pairs()
            .filter(|(key, _)| !is_quaint_only_parameter(key))
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();

        driver_url
            .query_pairs_mut()
            .clear()
            .extend_pairs(driver_params.iter().map(|(key, value)| (key.as_str(), value.as_str())));

        let mut config = driver_url
            .as_str()
            .parse::<Config>()
            .map_err(|error| Error::builder(ErrorKind::DatabaseUrlIsInvalid(error.to_string())).build())?;

        let mut pg_bouncer = false;
        let mut schema = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "pgbouncer" => pg_bouncer = value.parse().unwrap(),
                "schema" => schema = Some(value.into_owned()),
                _ => (),
            }
        }
        config.pgbouncer_mode(pg_bouncer);

        let schema = schema
            .as_deref()
            .unwrap_or(super::super::DEFAULT_KINGBASE_ORACLE_SCHEMA);
        config.search_path(format!("\"{}\"", schema.replace('"', "\"\"")));

        Ok(config)
    }

    fn is_quaint_only_parameter(key: &str) -> bool {
        matches!(
            key,
            "schema"
                | "pgbouncer"
                | "socket_timeout"
                | "connection_limit"
                | "pool_timeout"
                | "max_connection_lifetime"
                | "max_idle_connection_lifetime"
                | "statement_cache_size"
                | "sslcert"
                | "sslidentity"
                | "sslpassword"
                | "sslaccept"
        )
    }

    fn assert_same_config(value: &str) {
        let raw = Url::parse(value).unwrap();
        let expected = previous_config(&raw).unwrap();
        let url = KingbaseOracleUrl::new(raw).unwrap();
        let actual = url.to_config().unwrap();
        assert_eq!(actual, expected, "{value}");
        assert_eq!(actual.get_password(), expected.get_password(), "{value}");
        assert_eq!(actual.get_ssl_negotiation(), expected.get_ssl_negotiation(), "{value}");
        assert_eq!(actual.get_pgbouncer_mode(), expected.get_pgbouncer_mode(), "{value}");
        assert_eq!(actual.get_search_path(), expected.get_search_path(), "{value}");
    }

    #[test]
    fn preserves_authority_credentials_and_database() {
        for suffix in [
            "localhost/app",
            "localhost:54325/app",
            "localhost/",
            "localhost",
            "user:password@localhost/app",
            "us%40er:p%40ss%FF@localhost/my%20db",
            "user:@localhost/app",
            "[::1]/app",
            "%2Ftmp/app",
            "host1,host2/app",
            "ho%2525st/app",
        ] {
            assert_same_config(&format!("kingbase-oracle://{suffix}"));
        }
        #[cfg(unix)]
        assert_same_config("kingbase-oracle://%2Ftmp%FF/app");
    }

    #[test]
    fn preserves_driver_options_and_query_precedence() {
        for query in [
            "schema=tenant%22name&pgbouncer=true&socket_timeout=9&connection_limit=3&pool_timeout=4&max_connection_lifetime=5&max_idle_connection_lifetime=6",
            "user=override&password=new%2Bpassword&dbname=other&application_name=test+app&options=-c%20statement_timeout%3D1000",
            "user=first&user=last&password=first&password=last&dbname=first&dbname=last",
            "sslmode=disable&sslnegotiation=postgres&channel_binding=disable&target_session_attrs=any&load_balance_hosts=disable",
            "sslmode=prefer&sslnegotiation=direct&channel_binding=prefer&target_session_attrs=read-write&load_balance_hosts=random",
            "sslmode=require&channel_binding=require&target_session_attrs=read-only",
            "host=%2Ftmp&host=other&hostaddr=127.0.0.1,::1&port=1234,,5678",
            "host=first,second",
            "connect_timeout=10&tcp_user_timeout=20&keepalives=0&keepalives_idle=30&keepalives_interval=4&keepalives_retries=5",
            "connect_timeout=10&connect_timeout=0&tcp_user_timeout=20&tcp_user_timeout=-1&keepalives_idle=30&keepalives_idle=0&keepalives_interval=4&keepalives_interval=-1",
            "connect_timeout=0&tcp_user_timeout=0&keepalives=2&keepalives_idle=-1&keepalives_interval=0&keepalives_retries=0",
            "sslcert=root.pem&sslidentity=client.p12&sslpassword=secret&sslaccept=strict&statement_cache_size=ignored",
        ] {
            assert_same_config(&format!("kingbase-oracle://user:password@localhost:54325/app?{query}"));
        }
    }

    #[test]
    fn rejects_invalid_driver_options() {
        for query in [
            "sslmode=invalid",
            "sslnegotiation=invalid",
            "channel_binding=invalid",
            "target_session_attrs=invalid",
            "load_balance_hosts=invalid",
            "hostaddr=invalid",
            "port=65536",
            "tcp_user_timeout=invalid",
            "keepalives=-1",
            "keepalives_idle=invalid",
            "keepalives_interval=invalid",
            "keepalives_retries=-1",
            "unknown=value",
            "connect_timeout=9223372036854775808",
        ] {
            let raw = Url::parse(&format!("kingbase-oracle://localhost/app?{query}")).unwrap();
            assert!(previous_config(&raw).is_err(), "{query}");
            assert!(
                KingbaseOracleUrl::new(Url::parse(&format!("kingbase-oracle://localhost/app?{query}")).unwrap())
                    .and_then(|url| url.to_config())
                    .is_err(),
                "{query}"
            );
        }
        for value in [
            "kingbase-oracle://us%FFer@localhost/app",
            "kingbase-oracle://localhost/db%FF",
            "kingbase-oracle:///app",
        ] {
            let url = KingbaseOracleUrl::new(Url::parse(value).unwrap()).unwrap();
            assert!(previous_config(&url.url).is_err());
            assert!(url.to_config().is_err());
        }
    }

    #[test]
    fn preserves_extended_authority_and_options() {
        let mut cases = Vec::new();
        for authority in [
            "localhost",
            "user@localhost",
            "user:@localhost",
            ":password@localhost",
            "@localhost",
            "localhost,",
            ",localhost",
            "localhost,,other",
            "ho%2Cst",
            "ho%3Ast",
            "ho%3Fst",
            "ho%23st",
            "ho%40st",
            "[2001:db8::1]",
            "localhost:0",
        ] {
            for path in ["", "/", "/db", "/my%2Fdb", "/my%3Fdb", "/my%23db"] {
                cases.push(format!("kingbase-oracle://{authority}{path}"));
            }
        }
        for key in ["user", "password", "dbname", "options", "application_name", "host"] {
            for value in [
                "",
                "%20",
                "+",
                "%2B",
                "%25",
                "%2525",
                "%3F",
                "%26",
                "%23",
                "%40",
                "a%20b",
                "a+b",
                "a%2Bb",
                "%FF",
                "%2Ftmp%FF",
            ] {
                cases.push(format!("kingbase-oracle://localhost/db?{key}={value}"));
            }
        }
        for value in cases {
            let parsed = Url::parse(&value).unwrap();
            let old = previous_config(&parsed);
            let new = KingbaseOracleUrl::new(parsed).and_then(|url| url.to_config());
            match (old, new) {
                (Ok(old), Ok(new)) => {
                    assert_eq!(old, new, "{value}");
                    assert_eq!(old.get_password(), new.get_password(), "{value}");
                }
                (Err(_), Err(_)) => {}
                (old, new) => panic!("config acceptance changed: {value}: old={old:?} new={new:?}"),
            }
        }
    }

    #[test]
    fn preserves_driver_option_acceptance() {
        for key in [
            "sslmode",
            "sslnegotiation",
            "channel_binding",
            "target_session_attrs",
            "load_balance_hosts",
            "hostaddr",
            "port",
            "tcp_user_timeout",
            "keepalives",
            "keepalives_idle",
            "keepalives_interval",
            "keepalives_retries",
            "statement_cache_size",
            "application_name",
            "host",
            "unknown",
        ] {
            for value in [
                "",
                "0",
                "1",
                "-1",
                "+1",
                "%2B1",
                "65535",
                "65536",
                "9223372036854775808",
                "invalid",
            ] {
                let value = format!("kingbase-oracle://localhost/db?{key}={value}");
                let raw = Url::parse(&value).unwrap();
                let old = previous_config(&raw);
                let new = KingbaseOracleUrl::new(raw).and_then(|url| url.to_config());
                match (old, new) {
                    (Ok(old), Ok(new)) => assert_eq!(old, new, "{value}"),
                    (Err(_), Err(_)) => (),
                    (old, new) => panic!("config acceptance changed: {value}: old={old:?} new={new:?}"),
                }
            }
        }
    }

    #[test]
    fn preserves_pool_timeouts_and_tls_settings() {
        let url = KingbaseOracleUrl::new(Url::parse("kingbase-oracle://localhost/app?schema=tenant&pgbouncer=true&connect_timeout=7&socket_timeout=8&connection_limit=3&pool_timeout=4&max_connection_lifetime=5&max_idle_connection_lifetime=6&sslcert=root.pem&sslidentity=client.p12&sslpassword=secret&sslaccept=strict").unwrap()).unwrap();
        assert_eq!(url.schema(), Some("tenant"));
        assert!(url.pg_bouncer());
        assert_eq!(url.connect_timeout(), Some(Duration::from_secs(7)));
        assert_eq!(url.socket_timeout(), Some(Duration::from_secs(8)));
        assert_eq!(url.connection_limit(), Some(3));
        assert_eq!(url.pool_timeout(), Some(Duration::from_secs(4)));
        assert_eq!(url.max_connection_lifetime(), Some(Duration::from_secs(5)));
        assert_eq!(url.max_idle_connection_lifetime(), Some(Duration::from_secs(6)));
        assert_eq!(url.ssl_params().certificate_file.as_deref(), Some("root.pem"));
        assert_eq!(url.ssl_params().identity_file.as_deref(), Some("client.p12"));
        assert_eq!(url.ssl_params().identity_password.0.as_deref(), Some("secret"));
        assert_eq!(url.ssl_params().ssl_accept_mode, SslAcceptMode::Strict);
    }
}
