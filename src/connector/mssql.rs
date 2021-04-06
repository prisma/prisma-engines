mod conversion;
mod error;

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet, Transaction},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use connection_string::JdbcString;
use futures::lock::Mutex;
use std::{convert::TryFrom, fmt, str::FromStr, time::Duration};
use tiberius::*;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/// Wraps a connection url and exposes the parsing logic used by Quaint,
/// including default values.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "docs", doc(cfg(feature = "mssql")))]
pub struct MssqlUrl {
    connection_string: String,
    query_params: MssqlQueryParams,
}

/// TLS mode when connecting to SQL Server.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "docs", doc(cfg(feature = "mssql")))]
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
    connection_limit: Option<usize>,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    pool_timeout: Option<Duration>,
    transaction_isolation_level: Option<IsolationLevel>,
    max_connection_lifetime: Option<Duration>,
    max_idle_connection_lifetime: Option<Duration>,
}

#[derive(Debug, Clone, Copy)]
/// Controls the locking and row versioning behavior of Transact-SQL statements
/// issued by a connection to SQL Server. Read more from the [SQL Server
/// documentation].
///
/// [SQL Server documentation]: https://docs.microsoft.com/en-us/sql/t-sql/statements/set-transaction-isolation-level-transact-sql?view=sql-server-ver15
#[cfg_attr(feature = "docs", doc(cfg(feature = "mssql")))]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Snapshot,
    Serializable,
}

impl fmt::Display for IsolationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadUncommitted => write!(f, "READ UNCOMMITTED"),
            Self::ReadCommitted => write!(f, "READ COMMITTED"),
            Self::RepeatableRead => write!(f, "REPEATABLE READ"),
            Self::Snapshot => write!(f, "SNAPSHOT"),
            Self::Serializable => write!(f, "SERIALIZABLE"),
        }
    }
}

impl FromStr for IsolationLevel {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "READ UNCOMMITTED" => Ok(Self::ReadUncommitted),
            "READ COMMITTED" => Ok(Self::ReadCommitted),
            "REPEATABLE READ" => Ok(Self::RepeatableRead),
            "SNAPSHOT" => Ok(Self::Snapshot),
            "SERIALIZABLE" => Ok(Self::Serializable),
            _ => {
                let kind = ErrorKind::database_url_is_invalid(format!("Invalid isolation level `{}`", s));

                Err(Error::builder(kind).build())
            }
        }
    }
}

#[async_trait]
impl TransactionCapable for Mssql {
    async fn start_transaction(&self) -> crate::Result<Transaction<'_>> {
        Transaction::new(self, "BEGIN TRAN").await
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
    pub fn transaction_isolation_level(&self) -> Option<IsolationLevel> {
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
        self.socket_timeout
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
#[cfg_attr(feature = "docs", doc(cfg(feature = "mssql")))]
pub struct Mssql {
    client: Mutex<Client<Compat<TcpStream>>>,
    url: MssqlUrl,
    socket_timeout: Option<Duration>,
}

impl Mssql {
    /// Creates a new connection to SQL Server.
    #[tracing::instrument(name = "new_connection", skip(url))]
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
        };

        if let Some(isolation) = this.url.transaction_isolation_level() {
            this.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {}", isolation))
                .await?;
        };

        Ok(this)
    }
}

#[async_trait]
impl Queryable for Mssql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Mssql::build(q)?;
        self.query_raw(&sql, &params[..]).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Mssql::build(q)?;
        self.execute_raw(&sql, &params[..]).await
    }

    #[tracing::instrument(skip(self, params))]
    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("mssql.query_raw", sql, params, move || async move {
            let mut client = self.client.lock().await;
            let params = conversion::conv_params(params)?;

            let query = client.query(sql, params.as_slice());
            let mut results = super::timeout::socket(self.socket_timeout, query)
                .await?
                .into_results()
                .await?;

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

    #[tracing::instrument(skip(self, params))]
    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("mssql.execute_raw", sql, params, move || async move {
            let mut client = self.client.lock().await;
            let params = conversion::conv_params(params)?;

            let query = client.execute(sql, params.as_slice());
            let changes = super::timeout::socket(self.socket_timeout, query).await?.total();

            Ok(changes)
        })
        .await
    }

    #[tracing::instrument(skip(self))]
    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("mssql.raw_cmd", cmd, &[], move || async move {
            let mut client = self.client.lock().await;

            super::timeout::socket(self.socket_timeout, client.simple_query(cmd))
                .await?
                .into_results()
                .await?;

            Ok(())
        })
        .await
    }

    #[tracing::instrument(skip(self))]
    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT @@VERSION AS version"#;
        let rows = self.query_raw(query, &[]).await?;

        let version_string = rows
            .get(0)
            .and_then(|row| row.get("version").and_then(|version| version.to_string()));

        Ok(version_string)
    }

    fn begin_statement(&self) -> &'static str {
        "BEGIN TRAN"
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
            format!("jdbc:{}", input)
        }
    }

    fn parse_query_params(input: &str) -> crate::Result<MssqlQueryParams> {
        let mut conn = JdbcString::from_str(&Self::with_jdbc_prefix(input))?;

        let host = conn.server_name().map(|server_name| match conn.instance_name() {
            Some(instance_name) => format!(r#"{}\{}"#, server_name, instance_name),
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
            .map(|level| IsolationLevel::from_str(&level))
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
            .unwrap_or(EncryptMode::Off);

        let trust_server_certificate = props
            .remove("trustservercertificate")
            .or_else(|| props.remove("trust_server_certificate"))
            .map(|param| param.parse())
            .transpose()?
            .unwrap_or(false);

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
