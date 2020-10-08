mod conversion;
mod error;

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet, Transaction},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use futures::{future::FutureExt, lock::Mutex};
use lru_cache::LruCache;
use native_tls::{Certificate, Identity, TlsConnector};
use percent_encoding::percent_decode;
use postgres_native_tls::MakeTlsConnector;
use std::{
    borrow::{Borrow, Cow},
    fs,
    future::Future,
    time::Duration,
};
use tokio::time::timeout;
use tokio_postgres::{config::SslMode, Client, Config, Statement};
use url::Url;

pub(crate) const DEFAULT_SCHEMA: &str = "public";

#[derive(Clone)]
struct Hidden<T>(T);

impl<T> std::fmt::Debug for Hidden<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<HIDDEN>")
    }
}

struct PostgresClient(Client);

impl std::fmt::Debug for PostgresClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PostgresClient")
    }
}

/// A connector interface for the PostgreSQL database.
#[derive(Debug)]
pub struct PostgreSql {
    client: PostgresClient,
    pg_bouncer: bool,
    socket_timeout: Option<Duration>,
    statement_cache: Mutex<LruCache<String, Statement>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SslAcceptMode {
    Strict,
    AcceptInvalidCerts,
}

#[derive(Debug, Clone)]
pub struct SslParams {
    certificate_file: Option<String>,
    identity_file: Option<String>,
    identity_password: Hidden<Option<String>>,
    ssl_accept_mode: SslAcceptMode,
}

#[derive(Debug)]
struct SslAuth {
    certificate: Hidden<Option<Certificate>>,
    identity: Hidden<Option<Identity>>,
    ssl_accept_mode: SslAcceptMode,
}

impl Default for SslAuth {
    fn default() -> Self {
        Self {
            certificate: Hidden(None),
            identity: Hidden(None),
            ssl_accept_mode: SslAcceptMode::AcceptInvalidCerts,
        }
    }
}

impl SslAuth {
    fn certificate(&mut self, certificate: Certificate) -> &mut Self {
        self.certificate = Hidden(Some(certificate));
        self
    }

    fn identity(&mut self, identity: Identity) -> &mut Self {
        self.identity = Hidden(Some(identity));
        self
    }

    fn accept_mode(&mut self, mode: SslAcceptMode) -> &mut Self {
        self.ssl_accept_mode = mode;
        self
    }
}

impl SslParams {
    async fn into_auth(self) -> crate::Result<SslAuth> {
        let mut auth = SslAuth::default();
        auth.accept_mode(self.ssl_accept_mode);

        if let Some(ref cert_file) = self.certificate_file {
            let cert = fs::read(cert_file).map_err(|err| {
                Error::builder(ErrorKind::TlsError {
                    message: format!("cert file not found ({})", err),
                })
                .build()
            })?;

            auth.certificate(Certificate::from_pem(&cert)?);
        }

        if let Some(ref identity_file) = self.identity_file {
            let db = fs::read(identity_file).map_err(|err| {
                Error::builder(ErrorKind::TlsError {
                    message: format!("identity file not found ({})", err),
                })
                .build()
            })?;
            let password = self.identity_password.0.as_deref().unwrap_or("");
            let identity = Identity::from_pkcs12(&db, &password)?;

            auth.identity(identity);
        }

        Ok(auth)
    }
}

/// Wraps a connection url and exposes the parsing logic used by quaint, including default values.
#[derive(Debug, Clone)]
pub struct PostgresUrl {
    url: Url,
    query_params: PostgresUrlQueryParams,
}

impl PostgresUrl {
    /// Parse `Url` to `PostgresUrl`. Returns error for mistyped connection
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

    /// The database host. Taken first from the `host` query parameter, then
    /// from the `host` part of the URL. For socket connections, the query
    /// parameter must be used.
    ///
    /// If none of them are set, defaults to `localhost`.
    pub fn host(&self) -> &str {
        match (self.query_params.host.as_ref(), self.url.host_str()) {
            (Some(host), _) => host.as_str(),
            (None, Some("")) => "localhost",
            (None, None) => "localhost",
            (None, Some(host)) => host,
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
        &self.query_params.schema
    }

    /// Whether the pgbouncer mode is enabled.
    pub fn pg_bouncer(&self) -> bool {
        self.query_params.pg_bouncer
    }

    pub(crate) fn connect_timeout(&self) -> Option<Duration> {
        self.query_params.connect_timeout
    }

    pub(crate) fn cache(&self) -> LruCache<String, Statement> {
        if self.query_params.pg_bouncer {
            LruCache::new(0)
        } else {
            LruCache::new(self.query_params.statement_cache_size)
        }
    }

    fn parse_query_params(url: &Url) -> Result<PostgresUrlQueryParams, Error> {
        let mut connection_limit = None;
        let mut schema = String::from(DEFAULT_SCHEMA);
        let mut certificate_file = None;
        let mut identity_file = None;
        let mut identity_password = None;
        let mut ssl_accept_mode = SslAcceptMode::AcceptInvalidCerts;
        let mut ssl_mode = SslMode::Prefer;
        let mut host = None;
        let mut socket_timeout = None;
        let mut connect_timeout = None;
        let mut pg_bouncer = false;
        let mut statement_cache_size = 500;

        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "pgbouncer" => {
                    pg_bouncer = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                }
                "sslmode" => {
                    match v.as_ref() {
                        "disable" => ssl_mode = SslMode::Disable,
                        "prefer" => ssl_mode = SslMode::Prefer,
                        "require" => ssl_mode = SslMode::Require,
                        _ => {
                            #[cfg(not(feature = "tracing-log"))]
                            debug!("Unsupported ssl mode {}, defaulting to 'prefer'", v);
                            #[cfg(feature = "tracing-log")]
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
                            #[cfg(not(feature = "tracing-log"))]
                            debug!("Unsupported SSL accept mode {}, defaulting to `strict`", v);
                            #[cfg(feature = "tracing-log")]
                            tracing::debug!(
                                message = "Unsupported SSL accept mode, defaulting to `strict`",
                                mode = &*v
                            );

                            ssl_accept_mode = SslAcceptMode::Strict;
                        }
                    };
                }
                "schema" => {
                    schema = v.to_string();
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
                    connect_timeout = Some(Duration::from_secs(as_int));
                }
                _ => {
                    #[cfg(not(feature = "tracing-log"))]
                    trace!("Discarding connection string param: {}", k);
                    #[cfg(feature = "tracing-log")]
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
            ssl_mode,
            host,
            connect_timeout,
            socket_timeout,
            pg_bouncer,
            statement_cache_size,
        })
    }

    pub(crate) fn ssl_params(&self) -> &SslParams {
        &self.query_params.ssl_params
    }

    #[cfg(feature = "pooled")]
    pub(crate) fn connection_limit(&self) -> Option<usize> {
        self.query_params.connection_limit
    }

    pub(crate) fn to_config(&self) -> Config {
        let mut config = Config::new();

        config.user(self.username().borrow());
        config.password(self.password().borrow() as &str);
        config.host(self.host());
        config.port(self.port());
        config.dbname(self.dbname());
        config.pgbouncer_mode(self.query_params.pg_bouncer);

        if let Some(connect_timeout) = self.query_params.connect_timeout {
            config.connect_timeout(connect_timeout);
        };

        config.ssl_mode(self.query_params.ssl_mode);

        config
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresUrlQueryParams {
    ssl_params: SslParams,
    connection_limit: Option<usize>,
    schema: String,
    ssl_mode: SslMode,
    pg_bouncer: bool,
    host: Option<String>,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    statement_cache_size: usize,
}

impl PostgreSql {
    /// Create a new connection to the database.
    pub async fn new(url: PostgresUrl) -> crate::Result<Self> {
        let config = url.to_config();

        let mut tls_builder = TlsConnector::builder();

        {
            let ssl_params = url.ssl_params();
            let auth = ssl_params.to_owned().into_auth().await?;

            if let Some(certificate) = auth.certificate.0 {
                tls_builder.add_root_certificate(certificate);
            }

            tls_builder.danger_accept_invalid_certs(auth.ssl_accept_mode == SslAcceptMode::AcceptInvalidCerts);

            if let Some(identity) = auth.identity.0 {
                tls_builder.identity(identity);
            }
        }

        let tls = MakeTlsConnector::new(tls_builder.build()?);
        let (client, conn) = config.connect(tls).await?;

        tokio::spawn(conn.map(|r| match r {
            Ok(_) => (),
            Err(e) => {
                #[cfg(not(feature = "tracing-log"))]
                {
                    error!("Error in PostgreSQL connection: {:?}", e);
                }
                #[cfg(feature = "tracing-log")]
                {
                    tracing::error!("Error in PostgreSQL connection: {:?}", e);
                }
            }
        }));

        let schema = url.schema();

        // SET NAMES sets the client text encoding. It needs to be explicitly set for automatic
        // conversion to and from UTF-8 to happen server-side.
        //
        // Relevant docs: https://www.postgresql.org/docs/current/multibyte.html
        let session_variables = format!(
            r##"
            SET search_path = "{schema}";
            SET NAMES 'UTF8';
            "##,
            schema = schema
        );

        client.simple_query(session_variables.as_str()).await?;

        Ok(Self {
            client: PostgresClient(client),
            socket_timeout: url.query_params.socket_timeout,
            pg_bouncer: url.query_params.pg_bouncer,
            statement_cache: Mutex::new(url.cache()),
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

    async fn fetch_cached(&self, sql: &str) -> crate::Result<Statement> {
        let mut cache = self.statement_cache.lock().await;
        let capacity = cache.capacity();
        let stored = cache.len();

        match cache.get_mut(sql) {
            Some(stmt) => {
                #[cfg(not(feature = "tracing-log"))]
                {
                    trace!(
                        "CACHE HIT! (query: \"{}\", capacity: {}, stored: {})",
                        sql,
                        capacity,
                        stored,
                    );
                }
                #[cfg(feature = "tracing-log")]
                {
                    tracing::trace!(
                        message = "CACHE HIT!",
                        query = sql,
                        capacity = capacity,
                        stored = stored,
                    );
                }

                Ok(stmt.clone()) // arc'd
            }
            None => {
                #[cfg(not(feature = "tracing-log"))]
                {
                    trace!(
                        "CACHE MISS! (query: \"{}\", capacity: {}, stored: {}",
                        sql,
                        capacity,
                        stored,
                    );
                }
                #[cfg(feature = "tracing-log")]
                {
                    tracing::trace!(
                        message = "CACHE MISS!",
                        query = sql,
                        capacity = capacity,
                        stored = stored,
                    );
                }

                let stmt = self.timeout(self.client.0.prepare(sql)).await?;
                cache.insert(sql.to_string(), stmt.clone());
                Ok(stmt)
            }
        }
    }
}

impl TransactionCapable for PostgreSql {}

#[async_trait]
impl Queryable for PostgreSql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Postgres::build(q)?;
        self.query_raw(sql.as_str(), &params[..]).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Postgres::build(q)?;
        self.execute_raw(sql.as_str(), &params[..]).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("postgres.query_raw", sql, params, move || async move {
            let stmt = self.fetch_cached(sql).await?;

            let rows = self
                .timeout(self.client.0.query(&stmt, conversion::conv_params(params).as_slice()))
                .await?;

            let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());

            for row in rows {
                result.rows.push(row.get_result_row()?);
            }

            Ok(result)
        })
        .await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("postgres.execute_raw", sql, params, move || async move {
            let stmt = self.fetch_cached(sql).await?;

            let changes = self
                .timeout(self.client.0.execute(&stmt, conversion::conv_params(params).as_slice()))
                .await?;

            Ok(changes)
        })
        .await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("postgres.raw_cmd", cmd, &[], move || async move {
            self.timeout(self.client.0.simple_query(cmd)).await?;

            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT version()"#;
        let rows = self.query_raw(query, &[]).await?;

        let version_string = rows
            .get(0)
            .and_then(|row| row.get("version").and_then(|version| version.to_string()));

        Ok(version_string)
    }

    async fn server_reset_query(&self, tx: &Transaction<'_>) -> crate::Result<()> {
        if self.pg_bouncer {
            tx.raw_cmd("DEALLOCATE ALL").await
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(500, url.cache().capacity());
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

    #[tokio::test]
    async fn test_custom_search_path() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.query_pairs_mut().append_pair("schema", "musti-test");

        let client = Quaint::new(url.as_str()).await.unwrap();

        let result_set = client.query_raw("SHOW search_path", &[]).await.unwrap();
        let row = result_set.first().unwrap();

        assert_eq!(Some("\"musti-test\""), row[0].as_str());
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
                    assert_eq!("this_does_not_exist", db_name.as_str())
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
        assert!(matches!(err.kind(), ErrorKind::AuthenticationFailed { user } if user == "WRONG"));
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
}
