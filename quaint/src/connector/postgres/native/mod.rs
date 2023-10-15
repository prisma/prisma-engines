//! Definitions for the Postgres connector.
//! This module is not compatible with wasm32-* targets.
//! This module is only available with the `postgresql-native` feature.
mod cache;
pub(crate) mod column_type;
mod conversion;
mod error;
mod explain;
mod query;
mod websocket;

pub(crate) use crate::connector::postgres::url::PostgresNativeUrl;
use crate::connector::postgres::url::{Hidden, SslAcceptMode, SslParams};
use crate::connector::{
    ColumnType, DescribedColumn, DescribedParameter, DescribedQuery, IsolationLevel, Transaction, TransactionOptions,
    timeout,
};
use crate::error::NativeErrorKind;

use crate::ValueType;
use crate::prelude::DefaultTransaction;
use crate::{
    ast::{Query, Value},
    connector::{ResultSet, queryable::*, trace},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
pub use cache::QueryCache;
use cache::{CacheSettings, NoOpCache, PreparedStatementLruCache, TracingLruCache};
use column_type::PGColumnType;
use futures::StreamExt;
use futures::future::FutureExt;
use native_tls::{Certificate, Identity, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use postgres_types::{Kind as PostgresKind, Type as PostgresType};
use query::PreparedQuery;
use std::borrow::Cow;
use std::{
    fmt::{Debug, Display},
    fs,
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;
use tokio_postgres::{Client, Config, Statement, config::ChannelBinding};
use tracing_futures::WithSubscriber;
use websocket::connect_via_websocket;

/// The underlying postgres driver. Only available with the `expose-drivers`
/// Cargo feature.
#[cfg(feature = "expose-drivers")]
pub use tokio_postgres;

use super::PostgresWebSocketUrl;

pub(super) struct PostgresClient(Client);

impl Debug for PostgresClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PostgresClient")
    }
}

const DB_SYSTEM_NAME_POSTGRESQL: &str = "postgresql";
const DB_SYSTEM_NAME_COCKROACHDB: &str = "cockroachdb";

/// A connector interface for the PostgreSQL database.
///
/// # Type parameters
/// - `Cache`: The cache used for prepared queries.
#[derive(Debug)]
pub struct PostgreSql<Cache> {
    client: PostgresClient,
    handle: JoinHandle<()>,
    pg_bouncer: bool,
    single_use_connection: bool,
    socket_timeout: Option<Duration>,
    cache: Cache,
    is_healthy: AtomicBool,
    is_cockroachdb: bool,
    is_materialize: bool,
    db_system_name: &'static str,
}

/// A [`PostgreSql`] interface with the default caching strategy, which involves storing all
/// queries as prepared statements in an LRU cache.
pub type PostgreSqlWithDefaultCache = PostgreSql<PreparedStatementLruCache>;

/// A [`PostgreSql`] interface which executes all queries as prepared statements without caching
/// them.
pub type PostgreSqlWithNoCache = PostgreSql<NoOpCache>;

/// A [`PostgreSql`] interface with the tracing caching strategy, which involves storing query
/// type information in a dedicated LRU cache for applicable queries and not re-using any prepared
/// statements.
pub type PostgreSqlWithTracingCache = PostgreSql<TracingLruCache>;

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

impl PostgresNativeUrl {
    pub(crate) fn cache_settings(&self) -> CacheSettings {
        if self.query_params.pg_bouncer {
            CacheSettings { capacity: 0 }
        } else {
            CacheSettings {
                capacity: self.query_params.statement_cache_size,
            }
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

        config.user(self.username().as_ref());
        config.password(self.password().as_ref());
        config.host(self.host());
        config.port(self.port());
        config.dbname(self.dbname().as_ref());
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

impl PostgreSqlWithNoCache {
    /// Create a new websocket connection to managed database
    pub async fn new_with_websocket(url: PostgresWebSocketUrl) -> crate::Result<Self> {
        let (client, handle) = connect_via_websocket(url).await?;

        Ok(Self {
            client: PostgresClient(client),
            handle,
            socket_timeout: None,
            pg_bouncer: false,
            single_use_connection: false,
            cache: NoOpCache,
            is_healthy: AtomicBool::new(true),
            is_cockroachdb: false,
            is_materialize: false,
            db_system_name: DB_SYSTEM_NAME_POSTGRESQL,
        })
    }
}

impl<Cache: QueryCache> PostgreSql<Cache> {
    /// Create a new connection to the database.
    pub async fn new(url: PostgresNativeUrl, tls_manager: &MakeTlsConnectorManager) -> crate::Result<Self> {
        let config = url.to_config();

        let tls = tls_manager.get_connector().await?;
        let (client, conn) = timeout::connect(url.connect_timeout(), config.connect(tls)).await?;

        let is_cockroachdb = conn.parameter("crdb_version").is_some();
        let is_materialize = conn.parameter("mz_version").is_some();

        let handle = tokio::spawn(
            conn.map(|r| {
                if let Err(e) = r {
                    tracing::error!("Error in PostgreSQL connection: {e:?}");
                }
            })
            .with_current_subscriber(),
        );

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

        let db_system_name = if is_cockroachdb {
            DB_SYSTEM_NAME_COCKROACHDB
        } else {
            DB_SYSTEM_NAME_POSTGRESQL
        };

        Ok(Self {
            client: PostgresClient(client),
            handle,
            socket_timeout: url.socket_timeout(),
            pg_bouncer: url.pg_bouncer(),
            single_use_connection: url.single_use_connections(),
            cache: url.cache_settings().into(),
            is_healthy: AtomicBool::new(true),
            is_cockroachdb,
            is_materialize,
            db_system_name,
        })
    }

    /// The underlying tokio_postgres::Client. Only available with the
    /// `expose-drivers` Cargo feature. This is a lower level API when you need
    /// to get into database specific features.
    #[cfg(feature = "expose-drivers")]
    pub fn client(&self) -> &tokio_postgres::Client {
        &self.client.0
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

    // All credits go to sqlx: https://github.com/launchbadge/sqlx/blob/a892ebc6e283f443145f92bbc7fce4ae44547331/sqlx-postgres/src/connection/describe.rs#L417
    pub(crate) async fn get_nullable_for_columns(&self, stmt: &Statement) -> crate::Result<Vec<Option<bool>>> {
        let columns = stmt.columns();

        if columns.is_empty() {
            return Ok(vec![]);
        }

        let mut nullable_query = String::from("SELECT NOT pg_attribute.attnotnull as nullable FROM (VALUES ");
        let mut args = Vec::with_capacity(columns.len() * 3);

        for (i, (column, bind)) in columns.iter().zip((1..).step_by(3)).enumerate() {
            if !args.is_empty() {
                nullable_query += ", ";
            }

            nullable_query.push_str(&format!("(${}::int4, ${}::int8, ${}::int4)", bind, bind + 1, bind + 2));

            args.push(Value::from(i as i32));
            args.push(ValueType::Int64(column.table_oid().map(i64::from)).into());
            args.push(ValueType::Int32(column.column_id().map(i32::from)).into());
        }

        nullable_query.push_str(
            ") as col(idx, table_id, col_idx) \
            LEFT JOIN pg_catalog.pg_attribute \
                ON table_id IS NOT NULL \
               AND attrelid = table_id \
               AND attnum = col_idx \
            ORDER BY col.idx",
        );

        let nullable_query_result = self.query_raw(&nullable_query, &args).await?;
        let mut nullables = Vec::with_capacity(nullable_query_result.len());

        for row in nullable_query_result {
            let nullable = row.at(0).and_then(|v| v.as_bool());

            nullables.push(nullable)
        }

        // If the server is CockroachDB or Materialize, skip this step (#1248).
        if !self.is_cockroachdb && !self.is_materialize {
            // patch up our null inference with data from EXPLAIN
            let nullable_patch = self.nullables_from_explain(stmt).await?;

            for (nullable, patch) in nullables.iter_mut().zip(nullable_patch) {
                *nullable = patch.or(*nullable);
            }
        }

        Ok(nullables)
    }

    /// Infer nullability for columns of this statement using EXPLAIN VERBOSE.
    ///
    /// This currently only marks columns that are on the inner half of an outer join
    /// and returns `None` for all others.
    /// All credits go to sqlx: https://github.com/launchbadge/sqlx/blob/a892ebc6e283f443145f92bbc7fce4ae44547331/sqlx-postgres/src/connection/describe.rs#L482
    async fn nullables_from_explain(&self, stmt: &Statement) -> Result<Vec<Option<bool>>, Error> {
        use explain::{Explain, Plan, visit_plan};

        let mut explain = format!("EXPLAIN (VERBOSE, FORMAT JSON) EXECUTE {}", stmt.name());
        let params_len = stmt.params().len();
        let mut comma = false;

        if params_len > 0 {
            explain += "(";

            // fill the arguments list with NULL, which should theoretically be valid
            for _ in 0..params_len {
                if comma {
                    explain += ", ";
                }

                explain += "NULL";
                comma = true;
            }

            explain += ")";
        }

        let explain_result = self.query_raw(&explain, &[]).await?.into_single()?;
        let explains = explain_result
            .into_single()?
            .into_json()
            .map(serde_json::from_value::<[Explain; 1]>)
            .transpose()?;
        let explain = explains.as_ref().and_then(|x| x.first());

        let mut nullables = Vec::new();

        if let Some(Explain::Plan {
            plan: plan @ Plan {
                output: Some(outputs), ..
            },
        }) = explain
        {
            nullables.resize(outputs.len(), None);
            visit_plan(plan, outputs, &mut nullables);
        }

        Ok(nullables)
    }

    async fn query_raw_impl(
        &self,
        sql: &str,
        params: &[Value<'_>],
        types: &[PostgresType],
    ) -> crate::Result<ResultSet> {
        self.check_bind_variables_len(params)?;

        trace::query(self.db_system_name, sql, params, move || async move {
            let query = self.cache.get_query(&self.client.0, sql, types).await?;

            if query.param_types().len() != params.len() {
                let kind = ErrorKind::IncorrectNumberOfParameters {
                    expected: query.param_types().len(),
                    actual: params.len(),
                };

                return Err(Error::builder(kind).build());
            }

            let mut rows = Box::pin(
                self.perform_io(query.dispatch(&self.client.0, conversion::conv_params(params)))
                    .await?,
            );

            let types = query
                .column_types()
                .map(PGColumnType::from_pg_type)
                .map(ColumnType::from)
                .collect::<Vec<_>>();
            let names = query.column_names().map(|name| name.to_string()).collect::<Vec<_>>();
            let mut result = ResultSet::new(names, types, Vec::new());

            while let Some(row) = rows.next().await {
                result.rows.push(row?.get_result_row()?);
            }
            Ok(result)
        })
        .await
    }

    /// Closes the connection and waits for the connection task to complete.
    pub async fn close(self) {
        drop(self.client);
        self.handle.await.expect("connection task panicked")
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

#[async_trait]
impl<Cache: QueryCache> TransactionCapable for PostgreSql<Cache> {
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> crate::Result<Box<dyn Transaction + 'a>> {
        let opts = TransactionOptions::new(isolation, self.requires_isolation_first());

        Ok(Box::new(DefaultTransaction::new(self, opts).await?))
    }
}

#[async_trait]
impl<Cache: QueryCache> Queryable for PostgreSql<Cache> {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Postgres::build(q)?;

        self.query_raw(sql.as_str(), &params[..]).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.query_raw_impl(sql, params, &[]).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.query_raw_impl(sql, params, &conversion::params_to_types(params))
            .await
    }

    async fn describe_query(&self, sql: &str) -> crate::Result<DescribedQuery> {
        let stmt = self.cache.get_statement(&self.client.0, sql, &[]).await?;

        let mut columns: Vec<DescribedColumn> = Vec::with_capacity(stmt.columns().len());
        let mut parameters: Vec<DescribedParameter> = Vec::with_capacity(stmt.params().len());

        let enums_results = self
            .query_raw("SELECT oid, typname FROM pg_type WHERE typtype = 'e';", &[])
            .await?;

        fn find_enum_by_oid(enums: &ResultSet, enum_oid: u32) -> Option<&str> {
            enums.iter().find_map(|row| {
                let oid = row.get("oid")?.as_i64()?;
                let name = row.get("typname")?.as_str()?;

                if enum_oid == u32::try_from(oid).unwrap() {
                    Some(name)
                } else {
                    None
                }
            })
        }

        fn resolve_type(ty: &PostgresType, enums: &ResultSet) -> (ColumnType, Option<String>) {
            let column_type = ColumnType::from(ty);

            match ty.kind() {
                PostgresKind::Enum => {
                    let enum_name = find_enum_by_oid(enums, ty.oid())
                        .unwrap_or_else(|| panic!("Could not find enum with oid {}", ty.oid()));

                    (column_type, Some(enum_name.to_string()))
                }
                _ => (column_type, None),
            }
        }

        let nullables = self.get_nullable_for_columns(&stmt).await?;

        for (idx, (col, nullable)) in stmt.columns().iter().zip(nullables).enumerate() {
            let (typ, enum_name) = resolve_type(col.type_(), &enums_results);

            if col.name() == "?column?" {
                let kind = ErrorKind::QueryInvalidInput(format!(
                    "Invalid column name '?column?' for index {idx}. Your SQL query must explicitly alias that column name."
                ));

                return Err(Error::builder(kind).build());
            }

            columns.push(
                DescribedColumn::new_named(col.name(), typ)
                    .with_enum_name(enum_name)
                    // Make fields nullable by default if we can't infer nullability.
                    .is_nullable(nullable.unwrap_or(true)),
            );
        }

        for param in stmt.params() {
            let (typ, enum_name) = resolve_type(param, &enums_results);

            parameters.push(DescribedParameter::new_named(param.name(), typ).with_enum_name(enum_name));
        }

        let enum_names = enums_results
            .into_iter()
            .filter_map(|row| row.take("typname"))
            .filter_map(|v| v.to_string())
            .collect::<Vec<_>>();

        Ok(DescribedQuery {
            columns,
            parameters,
            enum_names: Some(enum_names),
        })
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Postgres::build(q)?;

        self.execute_raw(sql.as_str(), &params[..]).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.check_bind_variables_len(params)?;

        trace::query(self.db_system_name, sql, params, move || async move {
            let stmt = self.cache.get_statement(&self.client.0, sql, &[]).await?;

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

        trace::query(self.db_system_name, sql, params, move || async move {
            let types = conversion::params_to_types(params);
            let stmt = self.cache.get_statement(&self.client.0, sql, &types).await?;

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
        trace::query(self.db_system_name, cmd, &[], move || async move {
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

    /// Statement to begin a transaction
    fn begin_statement(&self, depth: u32) -> Cow<'static, str> {
        if depth > 1 {
            Cow::Owned(format!("SAVEPOINT savepoint{depth}"))
        } else {
            Cow::Borrowed("BEGIN")
        }
    }

    /// Statement to commit a transaction
    fn commit_statement(&self, depth: u32) -> Cow<'static, str> {
        if depth > 1 {
            Cow::Owned(format!("RELEASE SAVEPOINT savepoint{depth}"))
        } else {
            Cow::Borrowed("COMMIT")
        }
    }

    /// Statement to rollback a transaction
    fn rollback_statement(&self, depth: u32) -> Cow<'static, str> {
        if depth > 1 {
            Cow::Owned(format!("ROLLBACK TO SAVEPOINT savepoint{depth}"))
        } else {
            Cow::Borrowed("ROLLBACK")
        }
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

pub struct MakeTlsConnectorManager {
    url: PostgresNativeUrl,
    connector: OnceCell<MakeTlsConnector>,
}

impl MakeTlsConnectorManager {
    pub fn new(url: PostgresNativeUrl) -> Self {
        MakeTlsConnectorManager {
            url,
            connector: OnceCell::new(),
        }
    }

    pub async fn get_connector(&self) -> crate::Result<MakeTlsConnector> {
        self.connector
            .get_or_try_init(|| async {
                let mut tls_builder = TlsConnector::builder();

                {
                    let ssl_params = self.url.ssl_params();
                    let auth = ssl_params.to_owned().into_auth().await?;

                    if let Some(certificate) = auth.certificate.0 {
                        tls_builder.add_root_certificate(certificate);
                    }

                    tls_builder.danger_accept_invalid_certs(auth.ssl_accept_mode == SslAcceptMode::AcceptInvalidCerts);

                    if let Some(identity) = auth.identity.0 {
                        tls_builder.identity(identity);
                    }
                }

                let tls_connector = MakeTlsConnector::new(tls_builder.build()?);

                Ok(tls_connector)
            })
            .await
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::Queryable;
    pub(crate) use crate::connector::postgres::url::PostgresFlavour;
    use crate::tests::test_api::CRDB_CONN_STR;
    use crate::tests::test_api::postgres::CONN_STR;
    use url::Url;

    #[tokio::test]
    async fn test_custom_search_path_pg() {
        async fn test_path(schema_name: &str) -> Option<String> {
            let mut url = Url::parse(&CONN_STR).unwrap();
            url.query_pairs_mut().append_pair("schema", schema_name);

            let mut pg_url = PostgresNativeUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Postgres);

            let tls_manager = MakeTlsConnectorManager::new(pg_url.clone());

            let client = PostgreSqlWithDefaultCache::new(pg_url, &tls_manager).await.unwrap();

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

            let mut pg_url = PostgresNativeUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Postgres);

            let tls_manager = MakeTlsConnectorManager::new(pg_url.clone());

            let client = PostgreSqlWithDefaultCache::new(pg_url, &tls_manager).await.unwrap();

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

            let mut pg_url = PostgresNativeUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Cockroach);

            let tls_manager = MakeTlsConnectorManager::new(pg_url.clone());

            let client = PostgreSqlWithDefaultCache::new(pg_url, &tls_manager).await.unwrap();

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

            let mut pg_url = PostgresNativeUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Unknown);

            let tls_manager = MakeTlsConnectorManager::new(pg_url.clone());

            let client = PostgreSqlWithDefaultCache::new(pg_url, &tls_manager).await.unwrap();

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

            let mut pg_url = PostgresNativeUrl::new(url).unwrap();
            pg_url.set_flavour(PostgresFlavour::Unknown);

            let tls_manager = MakeTlsConnectorManager::new(pg_url.clone());

            let client = PostgreSqlWithDefaultCache::new(pg_url, &tls_manager).await.unwrap();

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
