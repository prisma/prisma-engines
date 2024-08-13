//! Definitions for the Postgres connector.
//! This module is not compatible with wasm32-* targets.
//! This module is only available with the `postgresql-native` feature.
pub(crate) mod column_type;
mod conversion;
mod error;

pub(crate) use crate::connector::postgres::url::PostgresUrl;
use crate::connector::postgres::url::{Hidden, SslAcceptMode, SslParams};
use crate::connector::{
    timeout, ColumnType, IsolationLevel, ParsedRawColumn, ParsedRawParameter, ParsedRawQuery, Transaction,
};
use crate::error::NativeErrorKind;

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use column_type::PGColumnType;
use futures::{future::FutureExt, lock::Mutex};
use lru_cache::LruCache;
use native_tls::{Certificate, Identity, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use postgres_types::{Kind as PostgresKind, Type as PostgresType};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{
    borrow::Borrow,
    fmt::{Debug, Display},
    fs,
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio_postgres::{config::ChannelBinding, Client, Config, Statement};

/// The underlying postgres driver. Only available with the `expose-drivers`
/// Cargo feature.
#[cfg(feature = "expose-drivers")]
pub use tokio_postgres;

struct PostgresClient(Client);

impl Debug for PostgresClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PostgresClient")
    }
}

/// A connector interface for the PostgreSQL database.
#[derive(Debug)]
pub struct PostgreSql {
    client: PostgresClient,
    pg_bouncer: bool,
    socket_timeout: Option<Duration>,
    statement_cache: Mutex<StatementCache>,
    is_healthy: AtomicBool,
}

/// Key uniquely representing an SQL statement in the prepared statements cache.
#[derive(PartialEq, Eq, Hash)]
pub(crate) struct StatementKey {
    /// Hash of a string with SQL query.
    sql: u64,
    /// Combined hash of types for all parameters from the query.
    types_hash: u64,
}

pub(crate) type StatementCache = LruCache<StatementKey, Statement>;

impl StatementKey {
    fn new(sql: &str, params: &[Value<'_>]) -> Self {
        Self {
            sql: {
                let mut hasher = DefaultHasher::new();
                sql.hash(&mut hasher);
                hasher.finish()
            },
            types_hash: {
                let mut hasher = DefaultHasher::new();
                for param in params {
                    std::mem::discriminant(&param.typed).hash(&mut hasher);
                }
                hasher.finish()
            },
        }
    }
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
                Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                    message: format!("cert file not found ({err})"),
                }))
                .build()
            })?;

            auth.certificate(Certificate::from_pem(&cert)?);
        }

        if let Some(ref identity_file) = self.identity_file {
            let db = fs::read(identity_file).map_err(|err| {
                Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                    message: format!("identity file not found ({err})"),
                }))
                .build()
            })?;
            let password = self.identity_password.0.as_deref().unwrap_or("");
            let identity = Identity::from_pkcs12(&db, password)?;

            auth.identity(identity);
        }

        Ok(auth)
    }
}

impl PostgresUrl {
    pub(crate) fn cache(&self) -> StatementCache {
        if self.query_params.pg_bouncer {
            StatementCache::new(0)
        } else {
            StatementCache::new(self.query_params.statement_cache_size)
        }
    }

    pub fn channel_binding(&self) -> ChannelBinding {
        self.query_params.channel_binding
    }

    /// On Postgres, we set the SEARCH_PATH and client-encoding through client connection parameters to save a network roundtrip on connection.
    /// We can't always do it for CockroachDB because it does not expect quotes for unsafe identifiers (https://github.com/cockroachdb/cockroach/issues/101328), which might change once the issue is fixed.
    /// To circumvent that problem, we only set the SEARCH_PATH through client connection parameters for Cockroach when the identifier is safe, so that the quoting does not matter.
    fn set_search_path(&self, config: &mut Config) {
        // PGBouncer does not support the search_path connection parameter.
        // https://www.pgbouncer.org/config.html#ignore_startup_parameters
        if self.query_params.pg_bouncer {
            return;
        }

        if let Some(schema) = &self.query_params.schema {
            if self.flavour().is_cockroach() && is_safe_identifier(schema) {
                config.search_path(CockroachSearchPath(schema).to_string());
            }

            if self.flavour().is_postgres() {
                config.search_path(PostgresSearchPath(schema).to_string());
            }
        }
    }

    pub(crate) fn to_config(&self) -> Config {
        let mut config = Config::new();

        config.user(self.username().borrow());
        config.password(self.password().borrow() as &str);
        config.host(self.host());
        config.port(self.port());
        config.dbname(self.dbname());
        config.pgbouncer_mode(self.query_params.pg_bouncer);

        if let Some(options) = self.options() {
            config.options(options);
        }

        if let Some(application_name) = self.application_name() {
            config.application_name(application_name);
        }

        if let Some(connect_timeout) = self.query_params.connect_timeout {
            config.connect_timeout(connect_timeout);
        }

        self.set_search_path(&mut config);

        config.ssl_mode(self.query_params.ssl_mode);

        config.channel_binding(self.query_params.channel_binding);

        config
    }
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
        let (client, conn) = timeout::connect(url.connect_timeout(), config.connect(tls)).await?;

        tokio::spawn(conn.map(|r| match r {
            Ok(_) => (),
            Err(e) => {
                tracing::error!("Error in PostgreSQL connection: {:?}", e);
            }
        }));

        // On Postgres, we set the SEARCH_PATH and client-encoding through client connection parameters to save a network roundtrip on connection.
        // We can't always do it for CockroachDB because it does not expect quotes for unsafe identifiers (https://github.com/cockroachdb/cockroach/issues/101328), which might change once the issue is fixed.
        // To circumvent that problem, we only set the SEARCH_PATH through client connection parameters for Cockroach when the identifier is safe, so that the quoting does not matter.
        // Finally, to ensure backward compatibility, we keep sending a database query in case the flavour is set to Unknown.
        if let Some(schema) = &url.query_params.schema {
            // PGBouncer does not support the search_path connection parameter.
            // https://www.pgbouncer.org/config.html#ignore_startup_parameters
            if url.query_params.pg_bouncer
                || url.flavour().is_unknown()
                || (url.flavour().is_cockroach() && !is_safe_identifier(schema))
            {
                let session_variables = format!(
                    r##"{set_search_path}"##,
                    set_search_path = SetSearchPath(url.query_params.schema.as_deref())
                );

                client.simple_query(session_variables.as_str()).await?;
            }
        }

        Ok(Self {
            client: PostgresClient(client),
            socket_timeout: url.query_params.socket_timeout,
            pg_bouncer: url.query_params.pg_bouncer,
            statement_cache: Mutex::new(url.cache()),
            is_healthy: AtomicBool::new(true),
        })
    }

    /// The underlying tokio_postgres::Client. Only available with the
    /// `expose-drivers` Cargo feature. This is a lower level API when you need
    /// to get into database specific features.
    #[cfg(feature = "expose-drivers")]
    pub fn client(&self) -> &tokio_postgres::Client {
        &self.client.0
    }

    async fn fetch_cached(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<Statement> {
        let statement_key = StatementKey::new(sql, params);
        let mut cache = self.statement_cache.lock().await;
        let capacity = cache.capacity();
        let stored = cache.len();

        match cache.get_mut(&statement_key) {
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

                let param_types = conversion::params_to_types(params);
                let stmt = self.perform_io(self.client.0.prepare_typed(sql, &param_types)).await?;

                cache.insert(statement_key, stmt.clone());

                Ok(stmt)
            }
        }
    }

    async fn perform_io<F, T>(&self, fut: F) -> crate::Result<T>
    where
        F: Future<Output = Result<T, tokio_postgres::Error>>,
    {
        match timeout::socket(self.socket_timeout, fut).await {
            Err(e) if e.is_closed() => {
                self.is_healthy.store(false, Ordering::SeqCst);
                Err(e)
            }
            res => res,
        }
    }

    fn check_bind_variables_len(&self, params: &[Value<'_>]) -> crate::Result<()> {
        if params.len() > i16::MAX as usize {
            // tokio_postgres would return an error here. Let's avoid calling the driver
            // and return an error early.
            let kind = ErrorKind::QueryInvalidInput(format!(
                "too many bind variables in prepared statement, expected maximum of {}, received {}",
                i16::MAX,
                params.len()
            ));
            Err(Error::builder(kind).build())
        } else {
            Ok(())
        }
    }
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

impl_default_TransactionCapable!(PostgreSql);

#[async_trait]
impl Queryable for PostgreSql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Postgres::build(q)?;

        self.query_raw(sql.as_str(), &params[..]).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.check_bind_variables_len(params)?;

        metrics::query("postgres.query_raw", sql, params, move || async move {
            let stmt = self.fetch_cached(sql, &[]).await?;

            if stmt.params().len() != params.len() {
                let kind = ErrorKind::IncorrectNumberOfParameters {
                    expected: stmt.params().len(),
                    actual: params.len(),
                };

                return Err(Error::builder(kind).build());
            }

            let rows = self
                .perform_io(self.client.0.query(&stmt, conversion::conv_params(params).as_slice()))
                .await?;

            let col_types = stmt
                .columns()
                .iter()
                .map(|c| PGColumnType::from_pg_type(c.type_()))
                .map(ColumnType::from)
                .collect::<Vec<_>>();
            let mut result = ResultSet::new(stmt.to_column_names(), col_types, Vec::new());

            for row in rows {
                result.rows.push(row.get_result_row()?);
            }

            Ok(result)
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.check_bind_variables_len(params)?;

        metrics::query("postgres.query_raw", sql, params, move || async move {
            let stmt = self.fetch_cached(sql, params).await?;

            if stmt.params().len() != params.len() {
                let kind = ErrorKind::IncorrectNumberOfParameters {
                    expected: stmt.params().len(),
                    actual: params.len(),
                };

                return Err(Error::builder(kind).build());
            }

            let col_types = stmt
                .columns()
                .iter()
                .map(|c| PGColumnType::from_pg_type(c.type_()))
                .map(ColumnType::from)
                .collect::<Vec<_>>();
            let rows = self
                .perform_io(self.client.0.query(&stmt, conversion::conv_params(params).as_slice()))
                .await?;

            let mut result = ResultSet::new(stmt.to_column_names(), col_types, Vec::new());

            for row in rows {
                result.rows.push(row.get_result_row()?);
            }

            Ok(result)
        })
        .await
    }

    async fn parse_raw_query(&self, sql: &str) -> crate::Result<ParsedRawQuery> {
        let stmt = self.fetch_cached(sql, &[]).await?;
        let mut columns: Vec<ParsedRawColumn> = Vec::with_capacity(stmt.columns().len());
        let mut parameters: Vec<ParsedRawParameter> = Vec::with_capacity(stmt.params().len());

        async fn infer_type(this: &PostgreSql, ty: &PostgresType) -> crate::Result<(ColumnType, Option<String>)> {
            let column_type = ColumnType::from(ty);

            match ty.kind() {
                PostgresKind::Enum => {
                    let enum_name = this
                        .query_raw("SELECT typname FROM pg_type WHERE oid = $1;", &[Value::int64(ty.oid())])
                        .await?
                        .into_single()?
                        .at(0)
                        .expect("could not find enum name")
                        .to_string()
                        .expect("enum name is not a string");

                    Ok((column_type, Some(enum_name)))
                }
                _ => Ok((column_type, None)),
            }
        }

        for col in stmt.columns() {
            let (typ, enum_name) = infer_type(self, col.type_()).await?;

            columns.push(ParsedRawColumn::new_named(col.name(), typ).with_enum_name(enum_name));
        }

        for param in stmt.params() {
            let (typ, enum_name) = infer_type(self, param).await?;

            parameters.push(ParsedRawParameter::new_named(param.name(), typ).with_enum_name(enum_name));
        }

        Ok(ParsedRawQuery { columns, parameters })
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Postgres::build(q)?;

        self.execute_raw(sql.as_str(), &params[..]).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.check_bind_variables_len(params)?;

        metrics::query("postgres.execute_raw", sql, params, move || async move {
            let stmt = self.fetch_cached(sql, &[]).await?;

            if stmt.params().len() != params.len() {
                let kind = ErrorKind::IncorrectNumberOfParameters {
                    expected: stmt.params().len(),
                    actual: params.len(),
                };

                return Err(Error::builder(kind).build());
            }

            let changes = self
                .perform_io(self.client.0.execute(&stmt, conversion::conv_params(params).as_slice()))
                .await?;

            Ok(changes)
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.check_bind_variables_len(params)?;

        metrics::query("postgres.execute_raw", sql, params, move || async move {
            let stmt = self.fetch_cached(sql, params).await?;

            if stmt.params().len() != params.len() {
                let kind = ErrorKind::IncorrectNumberOfParameters {
                    expected: stmt.params().len(),
                    actual: params.len(),
                };

                return Err(Error::builder(kind).build());
            }

            let changes = self
                .perform_io(self.client.0.execute(&stmt, conversion::conv_params(params).as_slice()))
                .await?;

            Ok(changes)
        })
        .await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("postgres.raw_cmd", cmd, &[], move || async move {
            self.perform_io(self.client.0.simple_query(cmd)).await?;
            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT version()"#;
        let rows = self.query_raw(query, &[]).await?;

        let version_string = rows
            .first()
            .and_then(|row| row.get("version").and_then(|version| version.to_string()));

        Ok(version_string)
    }

    fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    async fn server_reset_query(&self, tx: &dyn Transaction) -> crate::Result<()> {
        if self.pg_bouncer {
            tx.raw_cmd("DEALLOCATE ALL").await
        } else {
            Ok(())
        }
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
        false
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

#[cfg(test)]
mod tests {
    use super::*;
    pub(crate) use crate::connector::postgres::url::PostgresFlavour;
    use crate::connector::Queryable;
    use crate::tests::test_api::postgres::CONN_STR;
    use crate::tests::test_api::CRDB_CONN_STR;
    use url::Url;

    #[tokio::test]
    async fn test_custom_search_path_pg() {
        async fn test_path(schema_name: &str) -> Option<String> {
            let mut url = Url::parse(&CONN_STR).unwrap();
            url.query_pairs_mut().append_pair("schema", schema_name);

            let mut pg_url = PostgresUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Postgres);

            let client = PostgreSql::new(pg_url).await.unwrap();

            let result_set = client.query_raw("SHOW search_path", &[]).await.unwrap();
            let row = result_set.first().unwrap();

            row[0].typed.to_string()
        }

        // Safe
        assert_eq!(test_path("hello").await.as_deref(), Some("\"hello\""));
        assert_eq!(test_path("_hello").await.as_deref(), Some("\"_hello\""));
        assert_eq!(test_path("àbracadabra").await.as_deref(), Some("\"àbracadabra\""));
        assert_eq!(test_path("h3ll0").await.as_deref(), Some("\"h3ll0\""));
        assert_eq!(test_path("héllo").await.as_deref(), Some("\"héllo\""));
        assert_eq!(test_path("héll0$").await.as_deref(), Some("\"héll0$\""));
        assert_eq!(test_path("héll_0$").await.as_deref(), Some("\"héll_0$\""));

        // Not safe
        assert_eq!(test_path("Hello").await.as_deref(), Some("\"Hello\""));
        assert_eq!(test_path("hEllo").await.as_deref(), Some("\"hEllo\""));
        assert_eq!(test_path("$hello").await.as_deref(), Some("\"$hello\""));
        assert_eq!(test_path("hello!").await.as_deref(), Some("\"hello!\""));
        assert_eq!(test_path("hello#").await.as_deref(), Some("\"hello#\""));
        assert_eq!(test_path("he llo").await.as_deref(), Some("\"he llo\""));
        assert_eq!(test_path(" hello").await.as_deref(), Some("\" hello\""));
        assert_eq!(test_path("he-llo").await.as_deref(), Some("\"he-llo\""));
        assert_eq!(test_path("hÉllo").await.as_deref(), Some("\"hÉllo\""));
        assert_eq!(test_path("1337").await.as_deref(), Some("\"1337\""));
        assert_eq!(test_path("_HELLO").await.as_deref(), Some("\"_HELLO\""));
        assert_eq!(test_path("HELLO").await.as_deref(), Some("\"HELLO\""));
        assert_eq!(test_path("HELLO$").await.as_deref(), Some("\"HELLO$\""));
        assert_eq!(test_path("ÀBRACADABRA").await.as_deref(), Some("\"ÀBRACADABRA\""));

        for ident in RESERVED_KEYWORDS {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }

        for ident in RESERVED_TYPE_FUNCTION_NAMES {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }
    }

    #[tokio::test]
    async fn test_custom_search_path_pg_pgbouncer() {
        async fn test_path(schema_name: &str) -> Option<String> {
            let mut url = Url::parse(&CONN_STR).unwrap();
            url.query_pairs_mut().append_pair("schema", schema_name);
            url.query_pairs_mut().append_pair("pbbouncer", "true");

            let mut pg_url = PostgresUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Postgres);

            let client = PostgreSql::new(pg_url).await.unwrap();

            let result_set = client.query_raw("SHOW search_path", &[]).await.unwrap();
            let row = result_set.first().unwrap();

            row[0].typed.to_string()
        }

        // Safe
        assert_eq!(test_path("hello").await.as_deref(), Some("\"hello\""));
        assert_eq!(test_path("_hello").await.as_deref(), Some("\"_hello\""));
        assert_eq!(test_path("àbracadabra").await.as_deref(), Some("\"àbracadabra\""));
        assert_eq!(test_path("h3ll0").await.as_deref(), Some("\"h3ll0\""));
        assert_eq!(test_path("héllo").await.as_deref(), Some("\"héllo\""));
        assert_eq!(test_path("héll0$").await.as_deref(), Some("\"héll0$\""));
        assert_eq!(test_path("héll_0$").await.as_deref(), Some("\"héll_0$\""));

        // Not safe
        assert_eq!(test_path("Hello").await.as_deref(), Some("\"Hello\""));
        assert_eq!(test_path("hEllo").await.as_deref(), Some("\"hEllo\""));
        assert_eq!(test_path("$hello").await.as_deref(), Some("\"$hello\""));
        assert_eq!(test_path("hello!").await.as_deref(), Some("\"hello!\""));
        assert_eq!(test_path("hello#").await.as_deref(), Some("\"hello#\""));
        assert_eq!(test_path("he llo").await.as_deref(), Some("\"he llo\""));
        assert_eq!(test_path(" hello").await.as_deref(), Some("\" hello\""));
        assert_eq!(test_path("he-llo").await.as_deref(), Some("\"he-llo\""));
        assert_eq!(test_path("hÉllo").await.as_deref(), Some("\"hÉllo\""));
        assert_eq!(test_path("1337").await.as_deref(), Some("\"1337\""));
        assert_eq!(test_path("_HELLO").await.as_deref(), Some("\"_HELLO\""));
        assert_eq!(test_path("HELLO").await.as_deref(), Some("\"HELLO\""));
        assert_eq!(test_path("HELLO$").await.as_deref(), Some("\"HELLO$\""));
        assert_eq!(test_path("ÀBRACADABRA").await.as_deref(), Some("\"ÀBRACADABRA\""));

        for ident in RESERVED_KEYWORDS {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }

        for ident in RESERVED_TYPE_FUNCTION_NAMES {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }
    }

    #[tokio::test]
    async fn test_custom_search_path_crdb() {
        async fn test_path(schema_name: &str) -> Option<String> {
            let mut url = Url::parse(&CRDB_CONN_STR).unwrap();
            url.query_pairs_mut().append_pair("schema", schema_name);

            let mut pg_url = PostgresUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Cockroach);

            let client = PostgreSql::new(pg_url).await.unwrap();

            let result_set = client.query_raw("SHOW search_path", &[]).await.unwrap();
            let row = result_set.first().unwrap();

            row[0].typed.to_string()
        }

        // Safe
        assert_eq!(test_path("hello").await.as_deref(), Some("hello"));
        assert_eq!(test_path("_hello").await.as_deref(), Some("_hello"));
        assert_eq!(test_path("àbracadabra").await.as_deref(), Some("àbracadabra"));
        assert_eq!(test_path("h3ll0").await.as_deref(), Some("h3ll0"));
        assert_eq!(test_path("héllo").await.as_deref(), Some("héllo"));
        assert_eq!(test_path("héll0$").await.as_deref(), Some("héll0$"));
        assert_eq!(test_path("héll_0$").await.as_deref(), Some("héll_0$"));

        // Not safe
        assert_eq!(test_path("Hello").await.as_deref(), Some("\"Hello\""));
        assert_eq!(test_path("hEllo").await.as_deref(), Some("\"hEllo\""));
        assert_eq!(test_path("$hello").await.as_deref(), Some("\"$hello\""));
        assert_eq!(test_path("hello!").await.as_deref(), Some("\"hello!\""));
        assert_eq!(test_path("hello#").await.as_deref(), Some("\"hello#\""));
        assert_eq!(test_path("he llo").await.as_deref(), Some("\"he llo\""));
        assert_eq!(test_path(" hello").await.as_deref(), Some("\" hello\""));
        assert_eq!(test_path("he-llo").await.as_deref(), Some("\"he-llo\""));
        assert_eq!(test_path("hÉllo").await.as_deref(), Some("\"hÉllo\""));
        assert_eq!(test_path("1337").await.as_deref(), Some("\"1337\""));
        assert_eq!(test_path("_HELLO").await.as_deref(), Some("\"_HELLO\""));
        assert_eq!(test_path("HELLO").await.as_deref(), Some("\"HELLO\""));
        assert_eq!(test_path("HELLO$").await.as_deref(), Some("\"HELLO$\""));
        assert_eq!(test_path("ÀBRACADABRA").await.as_deref(), Some("\"ÀBRACADABRA\""));

        for ident in RESERVED_KEYWORDS {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }

        for ident in RESERVED_TYPE_FUNCTION_NAMES {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }
    }

    #[tokio::test]
    async fn test_custom_search_path_unknown_pg() {
        async fn test_path(schema_name: &str) -> Option<String> {
            let mut url = Url::parse(&CONN_STR).unwrap();
            url.query_pairs_mut().append_pair("schema", schema_name);

            let mut pg_url = PostgresUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Unknown);

            let client = PostgreSql::new(pg_url).await.unwrap();

            let result_set = client.query_raw("SHOW search_path", &[]).await.unwrap();
            let row = result_set.first().unwrap();

            row[0].typed.to_string()
        }

        // Safe
        assert_eq!(test_path("hello").await.as_deref(), Some("hello"));
        assert_eq!(test_path("_hello").await.as_deref(), Some("_hello"));
        assert_eq!(test_path("àbracadabra").await.as_deref(), Some("\"àbracadabra\""));
        assert_eq!(test_path("h3ll0").await.as_deref(), Some("h3ll0"));
        assert_eq!(test_path("héllo").await.as_deref(), Some("\"héllo\""));
        assert_eq!(test_path("héll0$").await.as_deref(), Some("\"héll0$\""));
        assert_eq!(test_path("héll_0$").await.as_deref(), Some("\"héll_0$\""));

        // Not safe
        assert_eq!(test_path("Hello").await.as_deref(), Some("\"Hello\""));
        assert_eq!(test_path("hEllo").await.as_deref(), Some("\"hEllo\""));
        assert_eq!(test_path("$hello").await.as_deref(), Some("\"$hello\""));
        assert_eq!(test_path("hello!").await.as_deref(), Some("\"hello!\""));
        assert_eq!(test_path("hello#").await.as_deref(), Some("\"hello#\""));
        assert_eq!(test_path("he llo").await.as_deref(), Some("\"he llo\""));
        assert_eq!(test_path(" hello").await.as_deref(), Some("\" hello\""));
        assert_eq!(test_path("he-llo").await.as_deref(), Some("\"he-llo\""));
        assert_eq!(test_path("hÉllo").await.as_deref(), Some("\"hÉllo\""));
        assert_eq!(test_path("1337").await.as_deref(), Some("\"1337\""));
        assert_eq!(test_path("_HELLO").await.as_deref(), Some("\"_HELLO\""));
        assert_eq!(test_path("HELLO").await.as_deref(), Some("\"HELLO\""));
        assert_eq!(test_path("HELLO$").await.as_deref(), Some("\"HELLO$\""));
        assert_eq!(test_path("ÀBRACADABRA").await.as_deref(), Some("\"ÀBRACADABRA\""));

        for ident in RESERVED_KEYWORDS {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }

        for ident in RESERVED_TYPE_FUNCTION_NAMES {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }
    }

    #[tokio::test]
    async fn test_custom_search_path_unknown_crdb() {
        async fn test_path(schema_name: &str) -> Option<String> {
            let mut url = Url::parse(&CONN_STR).unwrap();
            url.query_pairs_mut().append_pair("schema", schema_name);

            let mut pg_url = PostgresUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Unknown);

            let client = PostgreSql::new(pg_url).await.unwrap();

            let result_set = client.query_raw("SHOW search_path", &[]).await.unwrap();
            let row = result_set.first().unwrap();

            row[0].typed.to_string()
        }

        // Safe
        assert_eq!(test_path("hello").await.as_deref(), Some("hello"));
        assert_eq!(test_path("_hello").await.as_deref(), Some("_hello"));
        assert_eq!(test_path("àbracadabra").await.as_deref(), Some("\"àbracadabra\""));
        assert_eq!(test_path("h3ll0").await.as_deref(), Some("h3ll0"));
        assert_eq!(test_path("héllo").await.as_deref(), Some("\"héllo\""));
        assert_eq!(test_path("héll0$").await.as_deref(), Some("\"héll0$\""));
        assert_eq!(test_path("héll_0$").await.as_deref(), Some("\"héll_0$\""));

        // Not safe
        assert_eq!(test_path("Hello").await.as_deref(), Some("\"Hello\""));
        assert_eq!(test_path("hEllo").await.as_deref(), Some("\"hEllo\""));
        assert_eq!(test_path("$hello").await.as_deref(), Some("\"$hello\""));
        assert_eq!(test_path("hello!").await.as_deref(), Some("\"hello!\""));
        assert_eq!(test_path("hello#").await.as_deref(), Some("\"hello#\""));
        assert_eq!(test_path("he llo").await.as_deref(), Some("\"he llo\""));
        assert_eq!(test_path(" hello").await.as_deref(), Some("\" hello\""));
        assert_eq!(test_path("he-llo").await.as_deref(), Some("\"he-llo\""));
        assert_eq!(test_path("hÉllo").await.as_deref(), Some("\"hÉllo\""));
        assert_eq!(test_path("1337").await.as_deref(), Some("\"1337\""));
        assert_eq!(test_path("_HELLO").await.as_deref(), Some("\"_HELLO\""));
        assert_eq!(test_path("HELLO").await.as_deref(), Some("\"HELLO\""));
        assert_eq!(test_path("HELLO$").await.as_deref(), Some("\"HELLO$\""));
        assert_eq!(test_path("ÀBRACADABRA").await.as_deref(), Some("\"ÀBRACADABRA\""));

        for ident in RESERVED_KEYWORDS {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }

        for ident in RESERVED_TYPE_FUNCTION_NAMES {
            assert_eq!(test_path(ident).await.as_deref(), Some(format!("\"{ident}\"").as_str()));
        }
    }

    #[test]
    fn test_safe_ident() {
        // Safe
        assert!(is_safe_identifier("hello"));
        assert!(is_safe_identifier("_hello"));
        assert!(is_safe_identifier("àbracadabra"));
        assert!(is_safe_identifier("h3ll0"));
        assert!(is_safe_identifier("héllo"));
        assert!(is_safe_identifier("héll0$"));
        assert!(is_safe_identifier("héll_0$"));
        assert!(is_safe_identifier("disconnect_security_must_honor_connect_scope_one2m"));

        // Not safe
        assert!(!is_safe_identifier(""));
        assert!(!is_safe_identifier("Hello"));
        assert!(!is_safe_identifier("hEllo"));
        assert!(!is_safe_identifier("$hello"));
        assert!(!is_safe_identifier("hello!"));
        assert!(!is_safe_identifier("hello#"));
        assert!(!is_safe_identifier("he llo"));
        assert!(!is_safe_identifier(" hello"));
        assert!(!is_safe_identifier("he-llo"));
        assert!(!is_safe_identifier("hÉllo"));
        assert!(!is_safe_identifier("1337"));
        assert!(!is_safe_identifier("_HELLO"));
        assert!(!is_safe_identifier("HELLO"));
        assert!(!is_safe_identifier("HELLO$"));
        assert!(!is_safe_identifier("ÀBRACADABRA"));

        for ident in RESERVED_KEYWORDS {
            assert!(!is_safe_identifier(ident));
        }

        for ident in RESERVED_TYPE_FUNCTION_NAMES {
            assert!(!is_safe_identifier(ident));
        }
    }
}
