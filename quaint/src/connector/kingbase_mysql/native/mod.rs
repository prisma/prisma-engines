//! Native KingbaseES connector for the MySQL-compatible provider.
//!
//! Kingbase speaks the PostgreSQL wire protocol, but this connector is kept
//! separate from the PostgreSQL connector so provider-specific URL routing and
//! value conversion do not change the existing PostgreSQL path.

mod column_type;
mod conversion;
mod error;

use column_type::column_type_from_column;

use crate::{
    ast::{Query, Value},
    connector::{
        ColumnType, DescribedColumn, DescribedParameter, DescribedQuery, IsolationLevel, ResultSet, Transaction,
        queryable::*, timeout, trace,
    },
    error::{Error, ErrorKind, NativeErrorKind},
    visitor::KingbaseMysql as KingbaseMysqlVisitor,
};
use async_trait::async_trait;
use futures::{Future, StreamExt, future::FutureExt};
use kingbase_tokio_postgres::{
    Config,
    config::{ChannelBinding, LoadBalanceHosts, SslMode, SslNegotiation, TargetSessionAttrs},
};
use native_tls::{Certificate, Identity, TlsConnector};
use percent_encoding::percent_decode;
use std::{
    collections::HashMap,
    fs,
    sync::{
        RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::task::JoinHandle;
use tracing_futures::WithSubscriber;

use super::{
    KingbaseMysqlUrl,
    url::{SslAcceptMode, SslParams},
};
use kingbase_postgres_native_tls::MakeTlsConnector;
use kingbase_tokio_postgres::{Client, types::Type};

const DB_SYSTEM_NAME: &str = "kingbase";

const INDEX_NAMES_SQL: &str = "SELECT n.nspname, c.relname, d.description \
    FROM sys_catalog.sys_class c \
    JOIN sys_catalog.sys_namespace n ON n.oid=c.relnamespace \
    JOIN sys_catalog.sys_description d ON d.objoid=c.oid AND d.objsubid=0 \
    AND d.classoid=(SELECT classid FROM sys_catalog.sys_get_object_address(\
        'view', ARRAY['sys_catalog','sys_class'], ARRAY[]::text[])) \
    WHERE c.relkind IN ('i','I') AND d.description LIKE 'prisma:logical-index:%'";

/// A native Kingbase connection using the Kingbase MySQL SQL visitor.
pub struct KingbaseMysql {
    client: Client,
    handle: JoinHandle<()>,
    socket_timeout: Option<Duration>,
    pg_bouncer: bool,
    is_healthy: AtomicBool,
    logical_index_names: RwLock<HashMap<(String, String), String>>,
    transaction_active: AtomicBool,
}

impl KingbaseMysqlUrl {
    /// Build the native driver's configuration explicitly, as the PostgreSQL connector does.
    pub(crate) fn to_config(&self) -> crate::Result<Config> {
        // Hostless URLs could not receive the Kingbase default port in the
        // previous implementation; keep rejecting them instead of changing routing.
        if !self.url.has_host() {
            return Err(Error::builder(ErrorKind::DatabaseUrlIsInvalid("missing Kingbase URL host".into())).build());
        }
        let mut config = Config::new();
        let decode = |value: &str| {
            percent_decode(value.as_bytes())
                .decode_utf8()
                .map(|value| value.into_owned())
                .map_err(|_| {
                    Error::builder(ErrorKind::DatabaseUrlIsInvalid(
                        "invalid UTF-8 in connection URL".into(),
                    ))
                    .build()
                })
        };

        if !self.url.username().is_empty() || self.url.password().is_some() {
            config.user(decode(self.url.username())?);
        }
        if let Some(password) = self.url.password() {
            config.password(percent_decode(password.as_bytes()).collect::<Vec<_>>());
        }
        if let Some(host) = self.url.host_str().filter(|host| !host.is_empty()) {
            let hosts = host.split(',').collect::<Vec<_>>();
            for (index, host) in hosts.iter().enumerate() {
                let host = host
                    .strip_prefix('[')
                    .and_then(|host| host.strip_suffix(']'))
                    .unwrap_or(host);
                let host = percent_decode(host.as_bytes()).collect::<Vec<_>>();
                #[cfg(unix)]
                if host.first() == Some(&b'/') {
                    use std::os::unix::ffi::OsStrExt;
                    config.host_path(std::ffi::OsStr::from_bytes(&host));
                } else {
                    config.host(std::str::from_utf8(host.as_slice()).map_err(|_| {
                        Error::builder(ErrorKind::DatabaseUrlIsInvalid("invalid UTF-8 in host".into())).build()
                    })?);
                }
                #[cfg(not(unix))]
                {
                    let host = std::str::from_utf8(&host).map_err(|_| {
                        Error::builder(ErrorKind::DatabaseUrlIsInvalid("invalid UTF-8 in host".into())).build()
                    })?;
                    // The driver's non-Unix host parser also splits decoded commas.
                    for host in host.split(',') {
                        config.host(host);
                    }
                }
                // Preserve the driver's ordering for comma-separated authority hosts.
                config.port(if index + 1 == hosts.len() { self.port() } else { 5432 });
            }
        }
        if let Some(database) = self.dbname() {
            config.dbname(decode(database)?);
        }

        let params = &self.query_params;
        if let Some(user) = &params.user {
            config.user(user);
        }
        if let Some(password) = &params.password.0 {
            config.password(password);
        }
        if let Some(database) = &params.dbname {
            config.dbname(database);
        }
        if let Some(options) = &params.options {
            config.options(options);
        }
        if let Some(application_name) = &params.application_name {
            config.application_name(application_name);
        }
        for host in &params.hosts {
            #[cfg(unix)]
            config.host(host);
            #[cfg(not(unix))]
            for host in host.split(',') {
                config.host(host);
            }
        }
        for address in &params.hostaddrs {
            config.hostaddr(*address);
        }
        for port in &params.ports {
            config.port(*port);
        }
        if let Some(timeout) = params.driver_connect_timeout {
            config.connect_timeout(timeout);
        }
        if let Some(timeout) = params.tcp_user_timeout {
            config.tcp_user_timeout(timeout);
        }
        if let Some(keepalives) = params.keepalives {
            config.keepalives(keepalives);
        }
        if let Some(timeout) = params.keepalives_idle {
            config.keepalives_idle(timeout);
        }
        if let Some(timeout) = params.keepalives_interval {
            config.keepalives_interval(timeout);
        }
        if let Some(retries) = params.keepalives_retries {
            config.keepalives_retries(retries);
        }
        if let Some(value) = &params.ssl_mode {
            config.ssl_mode(match value.as_str() {
                "disable" => SslMode::Disable,
                "require" => SslMode::Require,
                _ => SslMode::Prefer,
            });
        }
        if let Some(value) = &params.ssl_negotiation {
            config.ssl_negotiation(match value.as_str() {
                "direct" => SslNegotiation::Direct,
                _ => SslNegotiation::Postgres,
            });
        }
        if let Some(value) = &params.target_session_attrs {
            config.target_session_attrs(match value.as_str() {
                "read-write" => TargetSessionAttrs::ReadWrite,
                "read-only" => TargetSessionAttrs::ReadOnly,
                _ => TargetSessionAttrs::Any,
            });
        }
        if let Some(value) = &params.channel_binding {
            config.channel_binding(match value.as_str() {
                "disable" => ChannelBinding::Disable,
                "require" => ChannelBinding::Require,
                _ => ChannelBinding::Prefer,
            });
        }
        if let Some(value) = &params.load_balance_hosts {
            config.load_balance_hosts(match value.as_str() {
                "random" => LoadBalanceHosts::Random,
                _ => LoadBalanceHosts::Disable,
            });
        }
        config.pgbouncer_mode(self.pg_bouncer());
        let schema = self.schema().unwrap_or(super::DEFAULT_KINGBASE_MYSQL_SCHEMA);
        config.search_path(format!("\"{}\"", schema.replace('"', "\"\"")));
        Ok(config)
    }
}

impl KingbaseMysql {
    pub async fn new(url: KingbaseMysqlUrl) -> crate::Result<Self> {
        let config = url.to_config()?;
        let tls = make_tls_connector(url.ssl_params())?;
        let (client, connection) = timeout::connect(url.connect_timeout(), config.connect(tls)).await?;

        let handle = tokio::spawn(
            connection
                .map(|result| {
                    if let Err(error) = result {
                        tracing::error!("Error in Kingbase connection: {error:?}");
                    }
                })
                .with_current_subscriber(),
        );

        Ok(Self {
            client,
            handle,
            socket_timeout: url.socket_timeout(),
            pg_bouncer: url.pg_bouncer(),
            is_healthy: AtomicBool::new(true),
            logical_index_names: RwLock::new(HashMap::new()),
            transaction_active: AtomicBool::new(false),
        })
    }

    async fn refresh_index_names(&self) {
        // Error decoration must never replace the original database error.
        if let Ok(rows) = timeout::socket(self.socket_timeout, self.client.query(INDEX_NAMES_SQL, &[])).await {
            let mut names = self.logical_index_names.write().unwrap();
            names.clear();
            for row in rows {
                let description: &str = row.get(2);
                if let Some(logical) = description
                    .strip_prefix("prisma:logical-index:")
                    .filter(|s| !s.is_empty())
                {
                    names.insert((row.get(0), row.get(1)), logical.to_owned());
                }
            }
        }
    }

    async fn map_driver_error(&self, error: kingbase_tokio_postgres::Error) -> Error {
        if let Some(db_error) = error.as_db_error()
            && db_error.code().code() == "23505"
            && let (Some(schema), Some(index)) = (db_error.schema(), db_error.constraint())
            && index.starts_with("_prisma_idx_")
        {
            let key = (schema.to_owned(), index.to_owned());
            if !self.transaction_active.load(Ordering::SeqCst) {
                // An autocommit connection may outlive migrations on another session.
                self.refresh_index_names().await;
            }
            let logical = self.logical_index_names.read().unwrap().get(&key).cloned();
            if let Some(logical) = logical {
                let mut mapped = super::error::KingbaseError::from(db_error);
                mapped.constraint = Some(logical);
                return mapped.into();
            }
        }
        error.into()
    }

    async fn perform_io<F, T>(&self, fut: F) -> crate::Result<T>
    where
        F: Future<Output = Result<T, kingbase_tokio_postgres::Error>>,
    {
        // Time out the user operation separately so error decoration cannot
        // replace an already-received constraint violation with a timeout.
        let result = timeout::socket(self.socket_timeout, async { Ok::<_, Error>(fut.await) }).await?;
        let result = match result {
            Ok(value) => Ok(value),
            Err(error) => Err(self.map_driver_error(error).await),
        };
        match result {
            Err(error) if error.is_closed() => {
                self.is_healthy.store(false, Ordering::SeqCst);
                Err(error)
            }
            result => result,
        }
    }

    fn check_bind_variables_len(&self, params: &[Value<'_>]) -> crate::Result<()> {
        if params.len() > i16::MAX as usize {
            Err(Error::builder(ErrorKind::QueryInvalidInput(format!(
                "too many bind variables in prepared statement, expected maximum of {}, received {}",
                i16::MAX,
                params.len()
            )))
            .build())
        } else {
            Ok(())
        }
    }

    async fn query_raw_impl(&self, sql: &str, params: &[Value<'_>], types: &[Type]) -> crate::Result<ResultSet> {
        self.check_bind_variables_len(params)?;
        let params = conversion::conv_params(params)?;

        let statement = self.perform_io(self.client.prepare_typed(sql, types)).await?;
        for typ in statement
            .params()
            .iter()
            .chain(statement.columns().iter().map(|column| column.type_()))
        {
            column_type::ensure_supported_type(typ)?;
        }

        if statement.params().len() != params.len() {
            return Err(Error::builder(ErrorKind::IncorrectNumberOfParameters {
                expected: statement.params().len(),
                actual: params.len(),
            })
            .build());
        }

        let mut rows = Box::pin(self.perform_io(self.client.query_raw(&statement, params)).await?);

        let names = statement
            .columns()
            .iter()
            .map(|column| column.name().to_owned())
            .collect::<Vec<_>>();
        let types = statement
            .columns()
            .iter()
            .map(column_type_from_column)
            .collect::<Vec<_>>();
        let mut result = ResultSet::new(names, types, Vec::new());

        while let Some(row) = rows.next().await {
            let row = match row {
                Ok(row) => row,
                Err(error) => return Err(self.map_driver_error(error).await),
            };
            result.rows.push(row.get_result_row()?);
        }

        Ok(result)
    }

    /// Closes the connection and waits for its background task.
    pub async fn close(self) {
        drop(self.client);
        self.handle.await.expect("Kingbase connection task panicked");
    }
}

fn make_tls_connector(ssl_params: &SslParams) -> crate::Result<MakeTlsConnector> {
    let mut tls_builder = TlsConnector::builder();

    if let Some(certificate_file) = &ssl_params.certificate_file {
        let certificate = fs::read(certificate_file).map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: format!("cert file not found ({error})"),
            }))
            .build()
        })?;
        let certificate = Certificate::from_pem(&certificate).map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: error.to_string(),
            }))
            .build()
        })?;
        tls_builder.add_root_certificate(certificate);
    }

    tls_builder.danger_accept_invalid_certs(ssl_params.ssl_accept_mode == SslAcceptMode::AcceptInvalidCerts);

    if let Some(identity_file) = &ssl_params.identity_file {
        let identity = fs::read(identity_file).map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: format!("identity file not found ({error})"),
            }))
            .build()
        })?;
        let password = ssl_params.identity_password.0.as_deref().unwrap_or("");
        let identity = Identity::from_pkcs12(&identity, password).map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: error.to_string(),
            }))
            .build()
        })?;
        tls_builder.identity(identity);
    }

    tls_builder
        .build()
        .map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: error.to_string(),
            }))
            .build()
        })
        .map(MakeTlsConnector::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connector::Queryable, single::Quaint, tests::test_api::kingbase_mysql::CONN_STR};

    #[tokio::test]
    async fn logical_index_names_survive_unique_errors_and_transaction_rollback() {
        let namespace = format!("kb_error_{}", uuid::Uuid::new_v4().simple());
        let mut url: url::Url = CONN_STR.parse().unwrap();
        url.query_pairs_mut().append_pair("schema", &namespace);
        let connection = KingbaseMysql::new(KingbaseMysqlUrl::new(url).unwrap()).await.unwrap();
        connection
            .raw_cmd(&format!("CREATE SCHEMA `{namespace}`"))
            .await
            .unwrap();
        let outcome: crate::Result<_> = async {
            connection
                .raw_cmd(
                    "CREATE TABLE logical_error_names(id INT PRIMARY KEY, value INT); \
                CREATE UNIQUE INDEX `_prisma_idx_error_probe` ON logical_error_names(value);\n\
                COMMENT ON INDEX `_prisma_idx_error_probe` IS 'prisma:logical-index:custom''unique'; \
                INSERT INTO logical_error_names VALUES(1,1)",
                )
                .await?;
            let mut errors = Vec::new();
            for transactional in [false, true] {
                let transaction = if transactional {
                    Some(connection.start_transaction(None).await?)
                } else {
                    None
                };
                let client: &dyn Queryable = transaction.as_ref().map(|t| t.as_queryable()).unwrap_or(&connection);
                if transactional {
                    client
                        .execute_raw("INSERT INTO logical_error_names VALUES(3,3)", &[])
                        .await?;
                }
                errors.push(
                    client
                        .query_raw("INSERT INTO logical_error_names VALUES(2,1)", &[])
                        .await
                        .unwrap_err(),
                );
                if let Some(transaction) = transaction {
                    transaction.rollback().await?;
                }
            }
            let count = connection
                .query_raw("SELECT COUNT(*)::INT FROM logical_error_names", &[])
                .await?
                .first()
                .unwrap()[0]
                .as_integer();
            Ok((errors, count))
        }
        .await;
        connection
            .raw_cmd(&format!("DROP SCHEMA `{namespace}` CASCADE"))
            .await
            .unwrap();
        let (errors, count) = outcome.unwrap();
        for error in errors {
            assert!(
                matches!(error.kind(), ErrorKind::UniqueConstraintViolation { constraint: crate::error::DatabaseConstraint::Index(name) } if name == "custom'unique"),
                "{error:?}"
            );
        }
        assert_eq!(count, Some(1));
    }

    #[tokio::test]
    async fn sql_arrays_are_rejected_but_json_arrays_and_in_queries_work() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        for value in [
            Value::array([1, 2]),
            Value::array(Vec::<i32>::new()),
            Value::null_array(),
            Value::null_enum_array(),
        ] {
            let params = [value];
            // Invalid SQL ensures unsupported arguments are rejected before prepare.
            assert!(
                connection
                    .query_raw("not SQL", &params)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
            assert!(
                connection
                    .query_raw_typed("not SQL", &params)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
            assert!(
                connection
                    .execute_raw("not SQL", &params)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
            assert!(
                connection
                    .execute_raw_typed("not SQL", &params)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
        }
        for sql in [
            "SELECT ARRAY[1,2] AS value",
            "SELECT NULL::integer[] AS value",
            "SELECT ARRAY[1] AS value WHERE false",
        ] {
            assert!(matches!(
                connection.query_raw(sql, &[]).await.unwrap_err().kind(),
                ErrorKind::UnsupportedColumnType { .. }
            ));
            assert!(connection.describe_query(sql).await.is_err());
        }
        let json = serde_json::json!([1, "123", null]);
        let result = connection
            .query_raw("SELECT CAST(? AS JSON) AS value", &[Value::json(json.clone())])
            .await
            .unwrap();
        assert_eq!(result.first().unwrap()[0].as_json(), Some(&json));
        let result = connection
            .query_raw("SELECT 2 IN (?,?) AS value", &[Value::int32(1), Value::int32(2)])
            .await
            .unwrap();
        assert_eq!(result.first().unwrap()[0].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn describe_query_reports_text_for_a_double_parameter() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();

        connection
            .raw_cmd("CREATE TEMPORARY TABLE `typed_sql_double` (`value` DOUBLE)")
            .await
            .unwrap();

        let described = connection
            .describe_query("SELECT `value` FROM `typed_sql_double` WHERE `value` < ?")
            .await
            .unwrap();

        assert_eq!(described.parameters.len(), 1);
        // Kingbase MySQL reports `?` as text in prepared-statement metadata even
        // when it is compared to a DOUBLE column. Typed SQL callers can declare
        // the intended parameter type explicitly with `@param {Float}`.
        assert_eq!(described.parameters[0].typ, ColumnType::Text);
    }
}

impl_default_TransactionCapable!(KingbaseMysql);

#[async_trait]
impl Queryable for KingbaseMysql {
    async fn query(&self, query: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = KingbaseMysqlVisitor::build(query)?;
        self.query_raw(sql.as_str(), &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.query_raw_impl(sql, params, &[]).await
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.query_raw_impl(sql, params, &conversion::params_to_types(params))
                .await
        })
        .await
    }

    async fn execute(&self, query: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = KingbaseMysqlVisitor::build(query)?;
        self.execute_raw(sql.as_str(), &params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.execute_raw_with_types(sql, params, &[]).await
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.execute_raw_with_types(sql, params, &conversion::params_to_types(params))
                .await
        })
        .await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        trace::query(DB_SYSTEM_NAME, cmd, &[], move || async move {
            let command = cmd.trim().trim_end_matches(';').trim();
            if command.eq_ignore_ascii_case("BEGIN") {
                // Read before BEGIN: after a constraint error this session cannot
                // run catalog queries until the caller rolls the transaction back.
                self.refresh_index_names().await;
            }
            let names = super::error::index_names_from_sql(cmd);
            let has_index_comments = !names.is_empty();
            if let Err(error) = self.perform_io(self.client.simple_query(cmd)).await {
                if let ErrorKind::UniqueConstraintViolation {
                    constraint: crate::error::DatabaseConstraint::Index(physical),
                } = error.kind()
                    && let Some((_, logical)) = names.iter().find(|(name, _)| name == physical)
                {
                    let mut mapped = Error::builder(ErrorKind::UniqueConstraintViolation {
                        constraint: crate::error::DatabaseConstraint::Index(logical.clone()),
                    });
                    if let Some(code) = error.original_code() {
                        mapped.set_original_code(code);
                    }
                    if let Some(message) = error.original_message() {
                        mapped.set_original_message(message);
                    }
                    return Err(mapped.build());
                }
                return Err(error);
            }
            if command.eq_ignore_ascii_case("BEGIN") {
                self.transaction_active.store(true, Ordering::SeqCst);
            } else if command.eq_ignore_ascii_case("COMMIT") || command.eq_ignore_ascii_case("ROLLBACK") {
                self.transaction_active.store(false, Ordering::SeqCst);
            }
            if has_index_comments {
                self.refresh_index_names().await;
            }
            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let result = self.query_raw("SELECT version()", &[]).await?;
        Ok(result
            .first()
            .and_then(|row| row.get("version").and_then(|value| value.to_string())))
    }

    async fn describe_query(&self, sql: &str) -> crate::Result<DescribedQuery> {
        let statement = self.perform_io(self.client.prepare_typed(sql, &[])).await?;
        for typ in statement
            .params()
            .iter()
            .chain(statement.columns().iter().map(|column| column.type_()))
        {
            column_type::ensure_supported_type(typ)?;
        }
        let columns = statement
            .columns()
            .iter()
            .map(|column| DescribedColumn::new_named(column.name(), column_type_from_column(column)).is_nullable(true))
            .collect();
        let parameters = statement
            .params()
            .iter()
            .enumerate()
            .map(|(idx, typ)| DescribedParameter::new_unnamed(idx, ColumnType::from(typ)))
            .collect();

        Ok(DescribedQuery {
            columns,
            parameters,
            enum_names: None,
        })
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
            .await
    }

    fn requires_isolation_first(&self) -> bool {
        false
    }
}

impl KingbaseMysql {
    async fn execute_raw_with_types(&self, sql: &str, params: &[Value<'_>], types: &[Type]) -> crate::Result<u64> {
        self.check_bind_variables_len(params)?;
        let params = conversion::conv_params(params)?;
        let statement = self.perform_io(self.client.prepare_typed(sql, types)).await?;
        for typ in statement
            .params()
            .iter()
            .chain(statement.columns().iter().map(|column| column.type_()))
        {
            column_type::ensure_supported_type(typ)?;
        }

        if statement.params().len() != params.len() {
            return Err(Error::builder(ErrorKind::IncorrectNumberOfParameters {
                expected: statement.params().len(),
                actual: params.len(),
            })
            .build());
        }

        self.perform_io(self.client.execute(&statement, params.as_slice()))
            .await
    }
}
