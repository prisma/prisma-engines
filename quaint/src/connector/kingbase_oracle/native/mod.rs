//! Native connection support for KingbaseES in Oracle-compatible mode.
//!
//! SQL AST queries are rendered by the dedicated Kingbase Oracle visitor and
//! executed with Oracle-compatible bind parameter types.

mod column_type;
mod conversion;
mod error;

use column_type::column_type_from_type;
use error::convert_driver_error;

use crate::{
    ast::{Query, Value},
    connector::{
        DescribedColumn, DescribedParameter, DescribedQuery, IsolationLevel, ResultSet, Transaction, queryable::*,
        trace,
    },
    error::{Error, ErrorKind, NativeErrorKind},
    visitor::{KingbaseOracle as KingbaseOracleVisitor, Visitor},
};
use async_trait::async_trait;
use futures::{Future, StreamExt, future::FutureExt};
use kingbase_postgres_native_tls::MakeTlsConnector;
use kingbase_tokio_postgres::{Client, types::Type};
use kingbase_tokio_postgres::{
    Config,
    config::{ChannelBinding, LoadBalanceHosts, SslMode, SslNegotiation, TargetSessionAttrs},
};
use native_tls::{Certificate, Identity, TlsConnector};
use percent_encoding::percent_decode;
use std::{
    fs,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::task::JoinHandle;
use tracing_futures::WithSubscriber;

use super::{
    KingbaseOracleUrl,
    url::{SslAcceptMode, SslParams},
};

const DB_SYSTEM_NAME: &str = "kingbase";

/// A native KingbaseES connection using Oracle-compatible database semantics.
pub struct KingbaseOracle {
    client: Client,
    handle: JoinHandle<()>,
    socket_timeout: Option<Duration>,
    pg_bouncer: bool,
    is_healthy: AtomicBool,
}

impl KingbaseOracleUrl {
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
        let schema = self.schema().unwrap_or(super::DEFAULT_KINGBASE_ORACLE_SCHEMA);
        config.search_path(format!("\"{}\"", schema.replace('"', "\"\"")));
        Ok(config)
    }
}

impl KingbaseOracle {
    pub async fn new(url: KingbaseOracleUrl) -> crate::Result<Self> {
        let config = url.to_config()?;
        let tls = make_tls_connector(url.ssl_params())?;
        let connect = config.connect(tls);
        let (client, connection) = match url.connect_timeout() {
            Some(duration) => tokio::time::timeout(duration, connect)
                .await
                .map_err(|_| Error::builder(ErrorKind::Native(NativeErrorKind::ConnectTimeout)).build())?
                .map_err(convert_driver_error)?,
            None => connect.await.map_err(convert_driver_error)?,
        };

        let handle = tokio::spawn(
            connection
                .map(|result| {
                    if let Err(error) = result {
                        tracing::error!("Error in Kingbase Oracle connection: {error:?}");
                    }
                })
                .with_current_subscriber(),
        );

        let connection = Self {
            client,
            handle,
            socket_timeout: url.socket_timeout(),
            pg_bouncer: url.pg_bouncer(),
            is_healthy: AtomicBool::new(true),
        };

        // `TIMESTAMP WITH LOCAL TIME ZONE` is returned over the wire as a
        // plain `TIMESTAMP`, localized to the session time zone. Prisma
        // represents DateTime values as UTC instants, so keep the session in
        // UTC before any value can be written or decoded.
        connection
            .perform_io(connection.client.batch_execute("SET TIME ZONE 'UTC'"))
            .await?;

        Ok(connection)
    }

    async fn perform_io<F, T>(&self, future: F) -> crate::Result<T>
    where
        F: Future<Output = Result<T, kingbase_tokio_postgres::Error>>,
    {
        let result = match self.socket_timeout {
            Some(duration) => tokio::time::timeout(duration, future)
                .await
                .map_err(|_| Error::builder(ErrorKind::SocketTimeout).build())?,
            None => future.await,
        }
        .map_err(convert_driver_error);

        match result {
            Err(error) if error.is_closed() => {
                self.is_healthy.store(false, Ordering::SeqCst);
                Err(error)
            }
            result => result,
        }
    }

    fn check_bind_variables_len(params: &[Value<'_>]) -> crate::Result<()> {
        if params.len() <= i16::MAX as usize {
            Ok(())
        } else {
            Err(Error::builder(ErrorKind::QueryInvalidInput(format!(
                "too many bind variables in prepared statement, expected maximum of {}, received {}",
                i16::MAX,
                params.len()
            )))
            .build())
        }
    }

    async fn query_raw_impl(&self, sql: &str, params: &[Value<'_>], types: &[Type]) -> crate::Result<ResultSet> {
        Self::check_bind_variables_len(params)?;

        let statement = self.perform_io(self.client.prepare_typed(sql, types)).await?;
        if statement.params().len() != params.len() {
            return Err(Error::builder(ErrorKind::IncorrectNumberOfParameters {
                expected: statement.params().len(),
                actual: params.len(),
            })
            .build());
        }

        let converted = conversion::convert_params(params);
        let parameter_refs = conversion::as_params(&converted);
        let mut rows = Box::pin(
            self.perform_io(self.client.query_raw(&statement, parameter_refs))
                .await?,
        );

        let names = statement
            .columns()
            .iter()
            .map(|column| column.name().to_owned())
            .collect::<Vec<_>>();
        let types = statement
            .columns()
            .iter()
            .map(|column| column_type_from_type(column.type_()))
            .collect::<Vec<_>>();
        let mut result = ResultSet::new(names, types, Vec::new());

        while let Some(row) = rows.next().await {
            result.rows.push(conversion::convert_row(&driver_result(row)?)?);
        }

        Ok(result)
    }

    async fn execute_raw_impl(&self, sql: &str, params: &[Value<'_>], types: &[Type]) -> crate::Result<u64> {
        Self::check_bind_variables_len(params)?;

        let statement = self.perform_io(self.client.prepare_typed(sql, types)).await?;
        if statement.params().len() != params.len() {
            return Err(Error::builder(ErrorKind::IncorrectNumberOfParameters {
                expected: statement.params().len(),
                actual: params.len(),
            })
            .build());
        }

        let converted = conversion::convert_params(params);
        let parameter_refs = conversion::as_params(&converted);
        self.perform_io(self.client.execute(&statement, &parameter_refs)).await
    }

    /// Closes the connection and waits for its background task.
    pub async fn close(self) {
        drop(self.client);
        self.handle.await.expect("Kingbase Oracle connection task panicked");
    }
}

fn driver_result<T>(result: Result<T, kingbase_tokio_postgres::Error>) -> crate::Result<T> {
    result.map_err(convert_driver_error)
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

impl_default_TransactionCapable!(KingbaseOracle);

#[async_trait]
impl Queryable for KingbaseOracle {
    async fn query(&self, query: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = KingbaseOracleVisitor::build(query)?;
        self.query_raw_typed(sql.as_str(), &params).await
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
        let (sql, params) = KingbaseOracleVisitor::build(query)?;
        self.execute_raw_typed(sql.as_str(), &params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.execute_raw_impl(sql, params, &[]).await
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.execute_raw_impl(sql, params, &conversion::params_to_types(params))
                .await
        })
        .await
    }

    async fn raw_cmd(&self, command: &str) -> crate::Result<()> {
        trace::query(DB_SYSTEM_NAME, command, &[], move || async move {
            self.perform_io(self.client.simple_query(command)).await?;
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
        let columns = statement
            .columns()
            .iter()
            .map(|column| {
                DescribedColumn::new_named(column.name(), column_type_from_type(column.type_())).is_nullable(true)
            })
            .collect();
        let parameters = statement
            .params()
            .iter()
            .enumerate()
            .map(|(index, typ)| DescribedParameter::new_unnamed(index, column_type_from_type(typ)))
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

    async fn server_reset_query(&self, transaction: &dyn Transaction) -> crate::Result<()> {
        if self.pg_bouncer {
            transaction.raw_cmd("DEALLOCATE ALL").await
        } else {
            Ok(())
        }
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        if !matches!(
            isolation_level,
            IsolationLevel::ReadCommitted | IsolationLevel::Serializable
        ) {
            return Err(Error::builder(ErrorKind::invalid_isolation_level(&isolation_level)).build());
        }

        self.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation_level}"))
            .await
    }

    fn requires_isolation_first(&self) -> bool {
        false
    }
}
