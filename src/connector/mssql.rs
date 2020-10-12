mod conversion;
mod error;

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet, Transaction},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    convert::TryFrom,
    fmt::{self, Write},
    future::Future,
    str::FromStr,
    time::Duration,
};
use tiberius::*;
use tokio::{net::TcpStream, time::timeout};
use tokio_util::compat::{Compat, Tokio02AsyncWriteCompatExt};

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
    connection_limit: Option<usize>,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    transaction_isolation_level: Option<IsolationLevel>,
}

#[derive(Debug, Clone, Copy)]
/// Controls the locking and row versioning behavior of Transact-SQL statements
/// issued by a connection to SQL Server. Read more from the [SQL Server
/// documentation].
///
/// [SQL Server documentation]: https://docs.microsoft.com/en-us/sql/t-sql/statements/set-transaction-isolation-level-transact-sql?view=sql-server-ver15
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
    pub fn connection_limit(&self) -> Option<usize> {
        self.query_params.connection_limit()
    }

    pub fn socket_timeout(&self) -> Option<Duration> {
        self.query_params.socket_timeout()
    }

    pub fn connect_timeout(&self) -> Option<Duration> {
        self.query_params.connect_timeout()
    }

    pub fn transaction_isolation_level(&self) -> Option<IsolationLevel> {
        self.query_params.transaction_isolation_level
    }

    pub fn dbname(&self) -> &str {
        self.query_params.database()
    }

    pub fn schema(&self) -> &str {
        self.query_params.schema()
    }

    pub fn host(&self) -> &str {
        self.query_params.host()
    }

    pub fn username(&self) -> Option<&str> {
        self.query_params.user()
    }

    pub fn port(&self) -> u16 {
        self.query_params.port()
    }
}

impl MssqlQueryParams {
    fn encrypt(&self) -> EncryptMode {
        self.encrypt
    }

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

    fn database(&self) -> &str {
        &self.database
    }

    fn schema(&self) -> &str {
        &self.schema
    }

    fn trust_server_certificate(&self) -> bool {
        self.trust_server_certificate
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
}

/// A connector interface for the PostgreSQL database.
#[derive(Debug)]
pub struct Mssql {
    client: Mutex<Client<Compat<TcpStream>>>,
    url: MssqlUrl,
    socket_timeout: Option<Duration>,
}

impl Mssql {
    pub async fn new(url: MssqlUrl) -> crate::Result<Self> {
        let config = Config::from_ado_string(&url.connection_string)?;
        let tcp = TcpStream::connect_named(&config).await?;
        let client = Client::connect(config, tcp.compat_write()).await?;
        let socket_timeout = url.socket_timeout();

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

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("mssql.query_raw", sql, params, move || async move {
            let mut client = self.client.lock().await;
            let params = conversion::conv_params(params)?;
            let query = client.query(sql, params.as_slice());

            let results = self.timeout(query).await?;

            let columns = results
                .columns()
                .unwrap_or(&[])
                .iter()
                .map(|c| c.name().to_string())
                .collect();

            let rows = results.into_first_result().await?;

            let mut result = ResultSet::new(columns, Vec::new());

            for row in rows {
                let mut values: Vec<Value<'_>> = Vec::with_capacity(row.len());

                for val in row.into_iter() {
                    values.push(Value::try_from(val)?);
                }

                result.rows.push(values);
            }

            Ok(result)
        })
        .await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("mssql.execute_raw", sql, params, move || async move {
            let mut client = self.client.lock().await;
            let params = conversion::conv_params(params)?;
            let query = client.execute(sql, params.as_slice());

            let changes = self.timeout(query).await?.total();

            Ok(changes)
        })
        .await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("mssql.raw_cmd", cmd, &[], move || async move {
            let mut client = self.client.lock().await;
            self.timeout(client.simple_query(cmd)).await?.into_results().await?;

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

    fn begin_statement(&self) -> &'static str {
        "BEGIN TRAN"
    }
}

impl MssqlUrl {
    pub fn new(jdbc_connection_string: &str) -> crate::Result<Self> {
        let query_params = Self::parse_query_params(jdbc_connection_string)?;
        let connection_string = Self::create_ado_net_string(&query_params)?;

        Ok(Self {
            connection_string,
            query_params,
        })
    }

    fn parse_query_params(input: &str) -> crate::Result<MssqlQueryParams> {
        let mut conn = connection_string::JdbcString::from_str(input)?;

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

        let connection_limit = props.remove("connectionlimit").map(|param| param.parse()).transpose()?;

        let transaction_isolation_level = props
            .remove("isolationlevel")
            .map(|level| IsolationLevel::from_str(&level))
            .transpose()?;

        let connect_timeout = props
            .remove("logintimeout")
            .or_else(|| props.remove("connecttimeout"))
            .or_else(|| props.remove("connectiontimeout"))
            .map(|param| param.parse().map(Duration::from_secs))
            .transpose()?;

        let socket_timeout = props
            .remove("sockettimeout")
            .map(|param| param.parse().map(Duration::from_secs))
            .transpose()?;

        let encrypt = props
            .remove("encrypt")
            .map(|param| EncryptMode::from_str(&param))
            .transpose()?
            .unwrap_or(EncryptMode::Off);

        let trust_server_certificate = props
            .remove("trustservercertificate")
            .map(|param| param.parse())
            .transpose()?
            .unwrap_or(false);

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
            transaction_isolation_level,
        })
    }

    fn create_ado_net_string(params: &MssqlQueryParams) -> crate::Result<String> {
        let mut buf = String::new();

        write!(&mut buf, "Server=tcp:{},{}", params.host(), params.port())?;
        write!(&mut buf, ";Encrypt={}", params.encrypt())?;
        write!(&mut buf, ";Database={}", params.database())?;

        write!(
            &mut buf,
            ";TrustServerCertificate={}",
            params.trust_server_certificate()
        )?;

        if let Some(user) = params.user() {
            write!(&mut buf, ";User ID={}", user)?;
        };

        if let Some(password) = params.password() {
            write!(&mut buf, ";Password={}", password)?;
        };

        Ok(buf)
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
        assert!(matches!(err.kind(), ErrorKind::AuthenticationFailed { user } if user == "WRONG"));
    }
}
