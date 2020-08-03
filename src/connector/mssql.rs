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
use std::{collections::HashMap, convert::TryFrom, fmt::Write, future::Future, time::Duration};
use tiberius::*;
use tokio::{net::TcpStream, time::timeout};
use tokio_util::compat::{Compat, Tokio02AsyncWriteCompatExt};
use url::Url;

#[derive(Debug, Clone)]
pub struct MssqlUrl {
    connection_string: String,
    query_params: MssqlQueryParams,
}

#[derive(Debug, Clone)]
pub(crate) struct MssqlQueryParams {
    encrypt: bool,
    port: Option<u16>,
    host: Option<String>,
    user: Option<String>,
    password: Option<String>,
    database: String,
    trust_server_certificate: bool,
    connection_limit: Option<usize>,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
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

    pub fn dbname(&self) -> &str {
        self.query_params.database()
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
    fn encrypt(&self) -> bool {
        self.encrypt
    }

    fn port(&self) -> u16 {
        self.port.unwrap_or(1433)
    }

    fn host(&self) -> &str {
        self.host.as_ref().map(|s| s.as_str()).unwrap_or("localhost")
    }

    fn user(&self) -> Option<&str> {
        self.user.as_ref().map(|s| s.as_str())
    }

    fn password(&self) -> Option<&str> {
        self.password.as_ref().map(|s| s.as_str())
    }

    fn database(&self) -> &str {
        &self.database
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

        Ok(Self {
            client: Mutex::new(client),
            url,
            socket_timeout,
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

    fn parse_query_params(jdbc_connection_string: &str) -> crate::Result<MssqlQueryParams> {
        let mut parts = jdbc_connection_string.split(';');

        match parts.next() {
            Some(host_part) => {
                let url = Url::parse(host_part)?;

                let params: crate::Result<HashMap<String, String>> = parts
                    .filter(|kv| kv != &"")
                    .map(|kv| kv.split("="))
                    .map(|mut split| {
                        let key = split
                            .next()
                            .ok_or_else(|| {
                                let kind = ErrorKind::conversion("Malformed connection string key");
                                Error::builder(kind).build()
                            })?
                            .trim();

                        let value = split.next().ok_or_else(|| {
                            let kind = ErrorKind::conversion("Malformed connection string value");
                            Error::builder(kind).build()
                        })?;

                        Ok((key.trim().to_lowercase(), value.trim().to_string()))
                    })
                    .collect();

                let mut params = params?;

                let host = url.host().map(|s| s.to_string());
                let port = url.port();
                let user = params.remove("user");
                let password = params.remove("password");
                let database = params.remove("database").unwrap_or_else(|| String::from("master"));
                let connection_limit = params.remove("connectionlimit").and_then(|param| param.parse().ok());

                let connect_timeout = params
                    .remove("logintimeout")
                    .or_else(|| params.remove("connecttimeout"))
                    .or_else(|| params.remove("connectiontimeout"))
                    .and_then(|param| param.parse::<u64>().ok())
                    .map(|secs| Duration::new(secs, 0));

                let socket_timeout = params
                    .remove("sockettimeout")
                    .and_then(|param| param.parse::<u64>().ok())
                    .map(|secs| Duration::new(secs, 0));

                let encrypt = params
                    .remove("encrypt")
                    .and_then(|param| param.parse().ok())
                    .unwrap_or(false);

                let trust_server_certificate = params
                    .remove("trustservercertificate")
                    .and_then(|param| param.parse().ok())
                    .unwrap_or(false);

                Ok(MssqlQueryParams {
                    encrypt,
                    port,
                    host,
                    user,
                    password,
                    database,
                    trust_server_certificate,
                    connection_limit,
                    socket_timeout,
                    connect_timeout,
                })
            }
            _ => {
                let kind = ErrorKind::conversion("Malformed connection string");
                Err(Error::builder(kind).build())
            }
        }
    }

    fn create_ado_net_string(params: &MssqlQueryParams) -> crate::Result<String> {
        let mut buf = String::new();

        write!(&mut buf, "Server=tcp:{},{}", params.host(), params.port())?;
        write!(&mut buf, ";Encrypt={}", params.encrypt())?;
        write!(&mut buf, ";Intial Catalog={}", params.database())?;

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
    use crate::tests::connector::mssql::CONN_STR;
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
