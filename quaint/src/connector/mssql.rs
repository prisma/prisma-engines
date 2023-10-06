mod conversion;
mod error;

use super::{IsolationLevel, Transaction, TransactionOptions};
use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, DefaultTransaction, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use connection_string::JdbcString;
use futures::lock::Mutex;
use std::{
    convert::TryFrom,
    fmt,
    future::Future,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tiberius::*;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/// The underlying SQL Server driver. Only available with the `expose-drivers` Cargo feature.
#[cfg(feature = "expose-drivers")]
pub use tiberius;

/// Wraps a connection url and exposes the parsing logic used by Quaint,
/// including default values.
#[derive(Debug, Clone)]
pub struct MssqlUrl {
    connection_string: String,
    query_params: MssqlQueryParams,
}

/// TLS mode when connecting to SQL Server.
#[derive(Debug, Clone, Copy)]
pub enum EncryptMode {
    /// All traffic is encrypted.
    On,
    /// Only the login credentials are encrypted.
    Off,
    /// Nothing is encrypted.
    DangerPlainText,
}

impl fmt::Display for EncryptMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::On => write!(f, "true"),
            Self::Off => write!(f, "false"),
            Self::DangerPlainText => write!(f, "DANGER_PLAINTEXT"),
        }
    }
}

impl FromStr for EncryptMode {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let mode = match s.parse::<bool>() {
            Ok(true) => Self::On,
            _ if s == "DANGER_PLAINTEXT" => Self::DangerPlainText,
            _ => Self::Off,
        };

        Ok(mode)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MssqlQueryParams {
    encrypt: EncryptMode,
    port: Option<u16>,
    host: Option<String>,
    user: Option<String>,
    password: Option<String>,
    database: String,
    schema: String,
    trust_server_certificate: bool,
    trust_server_certificate_ca: Option<String>,
    connection_limit: Option<usize>,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    pool_timeout: Option<Duration>,
    transaction_isolation_level: Option<IsolationLevel>,
    max_connection_lifetime: Option<Duration>,
    max_idle_connection_lifetime: Option<Duration>,
}

static SQL_SERVER_DEFAULT_ISOLATION: IsolationLevel = IsolationLevel::ReadCommitted;

#[async_trait]
impl TransactionCapable for Mssql {
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> crate::Result<Box<dyn Transaction + 'a>> {
        // Isolation levels in SQL Server are set on the connection and live until they're changed.
        // Always explicitly setting the isolation level each time a tx is started (either to the given value
        // or by using the default/connection string value) prevents transactions started on connections from
        // the pool to have unexpected isolation levels set.
        let isolation = isolation
            .or(self.url.query_params.transaction_isolation_level)
            .or(Some(SQL_SERVER_DEFAULT_ISOLATION));

        let opts = TransactionOptions::new(isolation, self.requires_isolation_first());

        Ok(Box::new(
            DefaultTransaction::new(self, self.begin_statement(), opts).await?,
        ))
    }
}

impl MssqlUrl {
    /// Maximum number of connections the pool can have (if used together with
    /// pooled Quaint).
    pub fn connection_limit(&self) -> Option<usize> {
        self.query_params.connection_limit()
    }

    /// A duration how long one query can take.
    pub fn socket_timeout(&self) -> Option<Duration> {
        self.query_params.socket_timeout()
    }

    /// A duration how long we can try to connect to the database.
    pub fn connect_timeout(&self) -> Option<Duration> {
        self.query_params.connect_timeout()
    }

    /// A pool check_out timeout.
    pub fn pool_timeout(&self) -> Option<Duration> {
        self.query_params.pool_timeout()
    }

    /// The isolation level of a transaction.
    fn transaction_isolation_level(&self) -> Option<IsolationLevel> {
        self.query_params.transaction_isolation_level
    }

    /// Name of the database.
    pub fn dbname(&self) -> &str {
        self.query_params.database()
    }

    /// The prefix which to use when querying database.
    pub fn schema(&self) -> &str {
        self.query_params.schema()
    }

    /// Database hostname.
    pub fn host(&self) -> &str {
        self.query_params.host()
    }

    /// The username to use when connecting to the database.
    pub fn username(&self) -> Option<&str> {
        self.query_params.user()
    }

    /// The password to use when connecting to the database.
    pub fn password(&self) -> Option<&str> {
        self.query_params.password()
    }

    /// The TLS mode to use when connecting to the database.
    pub fn encrypt(&self) -> EncryptMode {
        self.query_params.encrypt()
    }

    /// If true, we allow invalid certificates (self-signed, or otherwise
    /// dangerous) when connecting. Should be true only for development and
    /// testing.
    pub fn trust_server_certificate(&self) -> bool {
        self.query_params.trust_server_certificate()
    }

    /// Path to a custom server certificate file.
    pub fn trust_server_certificate_ca(&self) -> Option<&str> {
        self.query_params.trust_server_certificate_ca()
    }

    /// Database port.
    pub fn port(&self) -> u16 {
        self.query_params.port()
    }

    /// The JDBC connection string
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// The maximum connection lifetime
    pub fn max_connection_lifetime(&self) -> Option<Duration> {
        self.query_params.max_connection_lifetime()
    }

    /// The maximum idle connection lifetime
    pub fn max_idle_connection_lifetime(&self) -> Option<Duration> {
        self.query_params.max_idle_connection_lifetime()
    }
}

impl MssqlQueryParams {
    fn port(&self) -> u16 {
        self.port.unwrap_or(1433)
    }

    fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("localhost")
    }

    fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    fn encrypt(&self) -> EncryptMode {
        self.encrypt
    }

    fn trust_server_certificate(&self) -> bool {
        self.trust_server_certificate
    }

    fn trust_server_certificate_ca(&self) -> Option<&str> {
        self.trust_server_certificate_ca.as_deref()
    }

    fn database(&self) -> &str {
        &self.database
    }

    fn schema(&self) -> &str {
        &self.schema
    }

    fn socket_timeout(&self) -> Option<Duration> {
        self.socket_timeout
    }

    fn connect_timeout(&self) -> Option<Duration> {
        self.connect_timeout
    }

    fn connection_limit(&self) -> Option<usize> {
        self.connection_limit
    }

    fn pool_timeout(&self) -> Option<Duration> {
        self.pool_timeout
    }

    fn max_connection_lifetime(&self) -> Option<Duration> {
        self.max_connection_lifetime
    }

    fn max_idle_connection_lifetime(&self) -> Option<Duration> {
        self.max_idle_connection_lifetime
    }
}

/// A connector interface for the SQL Server database.
#[derive(Debug)]
pub struct Mssql {
    client: Mutex<Client<Compat<TcpStream>>>,
    url: MssqlUrl,
    socket_timeout: Option<Duration>,
    is_healthy: AtomicBool,
}

impl Mssql {
    /// Creates a new connection to SQL Server.
    pub async fn new(url: MssqlUrl) -> crate::Result<Self> {
        let config = Config::from_jdbc_string(&url.connection_string)?;
        let tcp = TcpStream::connect_named(&config).await?;
        let socket_timeout = url.socket_timeout();

        let connecting = async {
            match Client::connect(config, tcp.compat_write()).await {
                Ok(client) => Ok(client),
                Err(tiberius::error::Error::Routing { host, port }) => {
                    let mut config = Config::from_jdbc_string(&url.connection_string)?;
                    config.host(host);
                    config.port(port);

                    let tcp = TcpStream::connect_named(&config).await?;
                    Client::connect(config, tcp.compat_write()).await
                }
                Err(e) => Err(e),
            }
        };

        let client = super::timeout::connect(url.connect_timeout(), connecting).await?;

        let this = Self {
            client: Mutex::new(client),
            url,
            socket_timeout,
            is_healthy: AtomicBool::new(true),
        };

        if let Some(isolation) = this.url.transaction_isolation_level() {
            this.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation}"))
                .await?;
        };

        Ok(this)
    }

    /// The underlying Tiberius client. Only available with the `expose-drivers` Cargo feature.
    /// This is a lower level API when you need to get into database specific features.
    #[cfg(feature = "expose-drivers")]
    pub fn client(&self) -> &Mutex<Client<Compat<TcpStream>>> {
        &self.client
    }

    async fn perform_io<F, T>(&self, fut: F) -> crate::Result<T>
    where
        F: Future<Output = std::result::Result<T, tiberius::error::Error>>,
    {
        match super::timeout::socket(self.socket_timeout, fut).await {
            Err(e) if e.is_closed() => {
                self.is_healthy.store(false, Ordering::SeqCst);
                Err(e)
            }
            res => res,
        }
    }
}

#[async_trait]
impl Queryable for Mssql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Mssql::build(q)?;
        self.query_raw(&sql, &params[..]).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("mssql.query_raw", sql, params, move || async move {
            let mut client = self.client.lock().await;

            let mut query = tiberius::Query::new(sql);

            for param in params {
                query.bind(param);
            }

            let mut results = self.perform_io(query.query(&mut client)).await?.into_results().await?;

            match results.pop() {
                Some(rows) => {
                    let mut columns_set = false;
                    let mut columns = Vec::new();
                    let mut result_rows = Vec::with_capacity(rows.len());

                    for row in rows.into_iter() {
                        if !columns_set {
                            columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                            columns_set = true;
                        }

                        let mut values: Vec<Value<'_>> = Vec::with_capacity(row.len());

                        for val in row.into_iter() {
                            values.push(Value::try_from(val)?);
                        }

                        result_rows.push(values);
                    }

                    Ok(ResultSet::new(columns, result_rows))
                }
                None => Ok(ResultSet::new(Vec::new(), Vec::new())),
            }
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.query_raw(sql, params).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Mssql::build(q)?;
        self.execute_raw(&sql, &params[..]).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("mssql.execute_raw", sql, params, move || async move {
            let mut query = tiberius::Query::new(sql);

            for param in params {
                query.bind(param);
            }

            let mut client = self.client.lock().await;
            let changes = self.perform_io(query.execute(&mut client)).await?.total();

            Ok(changes)
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("mssql.raw_cmd", cmd, &[], move || async move {
            let mut client = self.client.lock().await;
            self.perform_io(client.simple_query(cmd)).await?.into_results().await?;
            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT @@VERSION AS version"#;
        let rows = self.query_raw(query, &[]).await?;

        let version_string = rows
            .get(0)
            .and_then(|row| row.get("version").and_then(|version| version.to_string()));

        Ok(version_string)
    }

    fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        self.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation_level}"))
            .await?;

        Ok(())
    }

    fn begin_statement(&self) -> &'static str {
        "BEGIN TRAN"
    }

    fn requires_isolation_first(&self) -> bool {
        true
    }
}

impl MssqlUrl {
    pub fn new(jdbc_connection_string: &str) -> crate::Result<Self> {
        let query_params = Self::parse_query_params(jdbc_connection_string)?;
        let connection_string = Self::with_jdbc_prefix(jdbc_connection_string);

        Ok(Self {
            connection_string,
            query_params,
        })
    }

    fn with_jdbc_prefix(input: &str) -> String {
        if input.starts_with("jdbc:sqlserver") {
            input.into()
        } else {
            format!("jdbc:{input}")
        }
    }

    fn parse_query_params(input: &str) -> crate::Result<MssqlQueryParams> {
        let mut conn = JdbcString::from_str(&Self::with_jdbc_prefix(input))?;

        let host = conn.server_name().map(|server_name| match conn.instance_name() {
            Some(instance_name) => format!(r#"{server_name}\{instance_name}"#),
            None => server_name.to_string(),
        });

        let port = conn.port();
        let props = conn.properties_mut();
        let user = props.remove("user");
        let password = props.remove("password");
        let database = props.remove("database").unwrap_or_else(|| String::from("master"));
        let schema = props.remove("schema").unwrap_or_else(|| String::from("dbo"));

        let connection_limit = props
            .remove("connectionlimit")
            .or_else(|| props.remove("connection_limit"))
            .map(|param| param.parse())
            .transpose()?;

        let transaction_isolation_level = props
            .remove("isolationlevel")
            .or_else(|| props.remove("isolation_level"))
            .map(|level| {
                IsolationLevel::from_str(&level).map_err(|_| {
                    let kind = ErrorKind::database_url_is_invalid(format!("Invalid isolation level `{level}`"));
                    Error::builder(kind).build()
                })
            })
            .transpose()?;

        let mut connect_timeout = props
            .remove("logintimeout")
            .or_else(|| props.remove("login_timeout"))
            .or_else(|| props.remove("connecttimeout"))
            .or_else(|| props.remove("connect_timeout"))
            .or_else(|| props.remove("connectiontimeout"))
            .or_else(|| props.remove("connection_timeout"))
            .map(|param| param.parse().map(Duration::from_secs))
            .transpose()?;

        match connect_timeout {
            None => connect_timeout = Some(Duration::from_secs(5)),
            Some(dur) if dur.as_secs() == 0 => connect_timeout = None,
            _ => (),
        }

        let mut pool_timeout = props
            .remove("pooltimeout")
            .or_else(|| props.remove("pool_timeout"))
            .map(|param| param.parse().map(Duration::from_secs))
            .transpose()?;

        match pool_timeout {
            None => pool_timeout = Some(Duration::from_secs(10)),
            Some(dur) if dur.as_secs() == 0 => pool_timeout = None,
            _ => (),
        }

        let socket_timeout = props
            .remove("sockettimeout")
            .or_else(|| props.remove("socket_timeout"))
            .map(|param| param.parse().map(Duration::from_secs))
            .transpose()?;

        let encrypt = props
            .remove("encrypt")
            .map(|param| EncryptMode::from_str(&param))
            .transpose()?
            .unwrap_or(EncryptMode::On);

        let trust_server_certificate = props
            .remove("trustservercertificate")
            .or_else(|| props.remove("trust_server_certificate"))
            .map(|param| param.parse())
            .transpose()?
            .unwrap_or(false);

        let trust_server_certificate_ca: Option<String> = props
            .remove("trustservercertificateca")
            .or_else(|| props.remove("trust_server_certificate_ca"));

        let mut max_connection_lifetime = props
            .remove("max_connection_lifetime")
            .map(|param| param.parse().map(Duration::from_secs))
            .transpose()?;

        match max_connection_lifetime {
            Some(dur) if dur.as_secs() == 0 => max_connection_lifetime = None,
            _ => (),
        }

        let mut max_idle_connection_lifetime = props
            .remove("max_idle_connection_lifetime")
            .map(|param| param.parse().map(Duration::from_secs))
            .transpose()?;

        match max_idle_connection_lifetime {
            None => max_idle_connection_lifetime = Some(Duration::from_secs(300)),
            Some(dur) if dur.as_secs() == 0 => max_idle_connection_lifetime = None,
            _ => (),
        }

        Ok(MssqlQueryParams {
            encrypt,
            port,
            host,
            user,
            password,
            database,
            schema,
            trust_server_certificate,
            trust_server_certificate_ca,
            connection_limit,
            socket_timeout,
            connect_timeout,
            pool_timeout,
            transaction_isolation_level,
            max_connection_lifetime,
            max_idle_connection_lifetime,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::test_api::mssql::CONN_STR;
    use crate::{error::*, single::Quaint};

    #[tokio::test]
    async fn should_map_wrong_credentials_error() {
        let url = CONN_STR.replace("user=SA", "user=WRONG");

        let res = Quaint::new(url.as_str()).await;
        assert!(res.is_err());

        let err = res.unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::AuthenticationFailed { user } if user == &Name::available("WRONG")));
    }
}
