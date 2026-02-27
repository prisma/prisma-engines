#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use crate::{
    connector::IsolationLevel,
    error::{Error, ErrorKind},
};
use connection_string::JdbcString;
use percent_encoding::percent_decode;
use std::{borrow::Cow, fmt, str::FromStr, time::Duration};

/// Wraps a connection url and exposes the parsing logic used by Quaint,
/// including default values.
#[derive(Debug, Clone)]
pub struct MssqlUrl {
    pub(crate) connection_string: String,
    pub(crate) query_params: MssqlQueryParams,
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
    pub(crate) encrypt: EncryptMode,
    pub(crate) port: Option<u16>,
    pub(crate) host: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) database: String,
    pub(crate) schema: String,
    pub(crate) trust_server_certificate: bool,
    pub(crate) trust_server_certificate_ca: Option<String>,
    pub(crate) connection_limit: Option<usize>,
    pub(crate) socket_timeout: Option<Duration>,
    pub(crate) connect_timeout: Option<Duration>,
    pub(crate) pool_timeout: Option<Duration>,
    pub(crate) transaction_isolation_level: Option<IsolationLevel>,
    pub(crate) max_connection_lifetime: Option<Duration>,
    pub(crate) max_idle_connection_lifetime: Option<Duration>,
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
    pub(crate) fn transaction_isolation_level(&self) -> Option<IsolationLevel> {
        self.query_params.transaction_isolation_level
    }

    /// Decoded database name. Defaults to `master`.
    pub fn dbname(&self) -> Cow<'_, str> {
        let db = self.query_params.database();
        match percent_decode(db.as_bytes()).decode_utf8() {
            Ok(decoded) => decoded,
            Err(_) => {
                tracing::warn!("Couldn't decode dbname to UTF-8, using the non-decoded version.");
                Cow::Borrowed(db)
            }
        }
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
    use super::*;
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

    #[test]
    fn should_decode_percent_encoded_dbname() {
        // Chinese characters: 测试库 (test database)
        let url = MssqlUrl::new("sqlserver://localhost:1433;database=%E6%B5%8B%E8%AF%95%E5%BA%93;user=SA;password=pass;trustServerCertificate=true").unwrap();
        assert_eq!("测试库", url.dbname());
    }

    #[test]
    fn should_decode_dbname_with_spaces() {
        let url = MssqlUrl::new(
            "sqlserver://localhost:1433;database=my%20database;user=SA;password=pass;trustServerCertificate=true",
        )
        .unwrap();
        assert_eq!("my database", url.dbname());
    }

    #[test]
    fn should_decode_dbname_with_special_characters() {
        // test-db_name
        let url = MssqlUrl::new(
            "sqlserver://localhost:1433;database=test%2Ddb%5Fname;user=SA;password=pass;trustServerCertificate=true",
        )
        .unwrap();
        assert_eq!("test-db_name", url.dbname());
    }

    #[test]
    fn should_return_master_as_default_dbname() {
        let url =
            MssqlUrl::new("sqlserver://localhost:1433;user=SA;password=pass;trustServerCertificate=true").unwrap();
        assert_eq!("master", url.dbname());
    }
}
