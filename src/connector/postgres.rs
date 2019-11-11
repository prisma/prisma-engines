mod conversion;
mod error;

use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{metrics, queryable::*, ResultSet, Transaction, DBIO},
    error::Error,
    visitor::{self, Visitor},
};
use async_std::{fs, sync::Mutex};
use futures::future::FutureExt;
use native_tls::{Certificate, Identity, TlsConnector};
use percent_encoding::percent_decode;
use postgres_native_tls::MakeTlsConnector;
use std::{borrow::Borrow, convert::TryFrom, time::Duration};
use tokio_postgres::{config::SslMode, Client, Config};
use url::Url;
use std::borrow::Cow;

pub(crate) const DEFAULT_SCHEMA: &str = "public";

/// A connector interface for the PostgreSQL database.
#[derive(DebugStub)]
pub struct PostgreSql {
    #[debug_stub = "postgres::Client"]
    client: Mutex<Client>,
}

#[derive(DebugStub)]
pub struct PostgresParams {
    pub connection_limit: u32,
    pub dbname: String,
    pub schema: String,
    pub config: Config,
    pub ssl_params: SslParams,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SslAcceptMode {
    Strict,
    AcceptInvalidCerts,
}

#[derive(DebugStub, Clone)]
pub struct SslParams {
    certificate_file: Option<String>,
    identity_file: Option<String>,
    #[debug_stub = "<HIDDEN>"]
    identity_password: Option<String>,
    ssl_accept_mode: SslAcceptMode,
}

#[derive(DebugStub)]
struct SslAuth {
    #[debug_stub = "<HIDDEN>"]
    certificate: Option<Certificate>,
    #[debug_stub = "<HIDDEN>"]
    identity: Option<Identity>,
    ssl_accept_mode: SslAcceptMode,
}

impl Default for SslAuth {
    fn default() -> Self {
        Self {
            certificate: None,
            identity: None,
            ssl_accept_mode: SslAcceptMode::AcceptInvalidCerts,
        }
    }
}

impl SslAuth {
    fn certificate(&mut self, certificate: Certificate) -> &mut Self {
        self.certificate = Some(certificate);
        self
    }

    fn identity(&mut self, identity: Identity) -> &mut Self {
        self.identity = Some(identity);
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
            let cert = fs::read(cert_file).await?;
            auth.certificate(Certificate::from_pem(&cert)?);
        }

        if let Some(ref identity_file) = self.identity_file {
            let db = fs::read(identity_file).await?;
            let password = self
                .identity_password
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("");
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
    pub fn new(url: Url) -> Result<Self, Error> {
        let query_params = Self::parse_query_params(&url)?;

        Ok(Self { url, query_params, })
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn username<'a>(&'a self) -> Cow<'a, str> {
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

    pub fn host(&self) -> &str {
        match (self.query_params.host.as_ref(), self.url.host_str()) {
            (Some(host), _) => host.as_str(),
            (None, Some("")) => "localhost",
            (None, None) => "localhost",
            (None, Some(host)) => host,
        }
    }

    pub fn dbname(&self) -> &str {
        match self.url.path_segments() {
            Some(mut segments) => segments.next().unwrap_or("postgres"),
            None => "postgres",
        }
    }

    pub fn password<'a>(&'a self) -> Cow<'a, str> {
        match self.url
            .password()
            .and_then(|pw| percent_decode(pw.as_bytes()).decode_utf8().ok())
            {
                Some(password) => {
                    password
                }
                None => {
                    self.url.password().unwrap_or("").into()
                }
            }
    }

    pub fn port(&self) -> u16 {
        self.url.port().unwrap_or(5432)
    }

    pub fn schema(&self) -> String {
        self.url.query_pairs().find(|(key, _value)| key == "schema").map(|(_key, value)| value.into_owned()).unwrap_or_else(|| DEFAULT_SCHEMA.to_string())
    }

    pub fn default_connection_limit() -> usize {
        num_cpus::get_physical() * 2 + 1
    }

    pub fn query_params(&self) -> &PostgresUrlQueryParams {
        &self.query_params
    }

    fn parse_query_params(url: &Url) -> Result<PostgresUrlQueryParams, Error> {
        let mut connection_limit = Self::default_connection_limit();
        let mut schema = String::from(DEFAULT_SCHEMA);
        let mut certificate_file = None;
        let mut identity_file = None;
        let mut identity_password = None;
        let mut ssl_accept_mode = SslAcceptMode::Strict;
        let mut ssl_mode = SslMode::Prefer;
        let mut host = None;

        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "sslmode" => {
                    match v.as_ref() {
                        "disable" => { ssl_mode = SslMode::Disable},
                        "prefer" => { ssl_mode = SslMode::Prefer },
                        "require" => { ssl_mode = SslMode::Require },
                        _ => {
                            #[cfg(not(feature = "tracing-log"))]
                            debug!("Unsupported ssl mode {}, defaulting to 'prefer'", v);
                            #[cfg(feature = "tracing-log")]
                            tracing::debug!(
                                message = "Unsupported SSL mode, defaulting to `prefer`",
                                mode = v.as_str()
                            );
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
                                mode = v.as_str()
                            );

                            ssl_accept_mode = SslAcceptMode::Strict;
                        }
                    };
                }
                "schema" => {
                    schema = v.to_string();
                }
                "connection_limit" => {
                    let as_int: usize = v.parse().map_err(|_| Error::InvalidConnectionArguments)?;
                    connection_limit = as_int;
                }
                "host" => {
                    host = Some(v.to_string());
                }
                _ => {
                    #[cfg(not(feature = "tracing-log"))]
                    trace!("Discarding connection string param: {}", k);
                    #[cfg(feature = "tracing-log")]
                    tracing::trace!(
                        message = "Discarding connection string param",
                        param = k.as_str()
                    );
                }
            };
        }

        Ok(PostgresUrlQueryParams {
            ssl_params: SslParams {
                certificate_file,
                identity_file,
                ssl_accept_mode,
                identity_password,
            },
            connection_limit,
            schema,
            ssl_mode,
            host,
        })

    }
}

#[derive(Debug, Clone)]
pub struct PostgresUrlQueryParams {
    ssl_params: SslParams,
    connection_limit: usize,
    schema: String,
    ssl_mode: SslMode,
    host: Option<String>,
}

impl TryFrom<Url> for PostgresParams {
    type Error = Error;

    fn try_from(url: Url) -> crate::Result<Self> {
        let url = PostgresUrl::new(url)?;

        let mut config = Config::new();

        config.user(url.username().borrow());
        config.password(url.password().borrow() as &str);
        config.host(url.host());
        config.port(url.port());
        config.dbname(url.dbname());
        config.connect_timeout(Duration::from_millis(5000));

        let dbname = url.dbname().to_string();
        let query_params = url.query_params;
        config.ssl_mode(query_params.ssl_mode);

        Ok(Self {
            connection_limit: u32::try_from(query_params.connection_limit).unwrap(),
            schema: query_params.schema,
            config,
            dbname,
            ssl_params: query_params.ssl_params,
        })
    }
}

impl PostgreSql {
    pub async fn new(
        config: Config,
        schema: Option<String>,
        ssl_params: Option<SslParams>,
    ) -> crate::Result<Self> {
        let mut tls_builder = TlsConnector::builder();

        if let Some(params) = ssl_params {
            let auth = params.into_auth().await?;

            if let Some(certificate) = auth.certificate {
                tls_builder.add_root_certificate(certificate);
            }

            tls_builder.danger_accept_invalid_certs(
                auth.ssl_accept_mode == SslAcceptMode::AcceptInvalidCerts,
            );

            if let Some(identity) = auth.identity {
                tls_builder.identity(identity);
            }
        };

        let tls = MakeTlsConnector::new(tls_builder.build()?);
        let (client, conn) = config.connect(tls).await?;
        tokio::spawn(conn.map(|r| r.unwrap()));

        let schema = schema.unwrap_or_else(|| String::from(DEFAULT_SCHEMA));
        let path = format!("SET search_path = \"{}\"", schema);
        client.execute(path.as_str(), &[]).await?;

        Ok(Self {
            client: Mutex::new(client),
        })
    }

    pub async fn from_params(params: PostgresParams) -> crate::Result<Self> {
        Self::new(params.config, Some(params.schema), Some(params.ssl_params)).await
    }

    pub async fn from_url(url: &str) -> crate::Result<Self> {
        Self::from_params(PostgresParams::try_from(Url::parse(url)?)?).await
    }

    fn execute_and_get_id<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ParameterizedValue],
    ) -> DBIO<'a, Option<Id>> {
        metrics::query("postgres.execute", sql, params, move || {
            async move {
                let client = self.client.lock().await;
                let stmt = client.prepare(sql).await?;
                let rows = client
                    .query(&stmt, conversion::conv_params(params).as_slice())
                    .await?;

                let id: Option<Id> = rows.into_iter().rev().next().map(|row| {
                    let id: Id = row.get(0);
                    id
                });

                Ok(id)
            }
        })
    }
}

impl Queryable for PostgreSql {
    fn execute<'a>(&'a self, q: Query<'a>) -> DBIO<'a, Option<Id>> {
        DBIO::new(async move {
            let (sql, params) = visitor::Postgres::build(q);
            self.execute_and_get_id(&sql, &params).await
        })
    }

    fn query<'a>(&'a self, q: Query<'a>) -> DBIO<'a, ResultSet> {
        let (sql, params) = visitor::Postgres::build(q);

        DBIO::new(async move { self.query_raw(sql.as_str(), &params[..]).await })
    }

    fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ParameterizedValue],
    ) -> DBIO<'a, ResultSet> {
        metrics::query("postgres.query_raw", sql, params, move || {
            async move {
                let client = self.client.lock().await;
                let stmt = client.prepare(sql).await?;
                let rows = client
                    .query(&stmt, conversion::conv_params(params).as_slice())
                    .await?;

                let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());

                for row in rows {
                    result.rows.push(row.to_result_row()?);
                }

                Ok(result)
            }
        })
    }

    fn execute_raw<'a>(&'a self, sql: &'a str, params: &'a [ParameterizedValue]) -> DBIO<'a, u64> {
        metrics::query("postgres.execute_raw", sql, params, move || {
            async move {
                let client = self.client.lock().await;
                let stmt = client.prepare(sql).await?;
                let changes = client
                    .execute(&stmt, conversion::conv_params(params).as_slice())
                    .await?;

                Ok(changes)
            }
        })
    }

    fn turn_off_fk_constraints(&self) -> DBIO<()> {
        DBIO::new(async move {
            self.query_raw("SET CONSTRAINTS ALL DEFERRED", &[]).await?;
            Ok(())
        })
    }

    fn turn_on_fk_constraints(&self) -> DBIO<()> {
        DBIO::new(async move {
            self.query_raw("SET CONSTRAINTS ALL IMMEDIATE", &[]).await?;
            Ok(())
        })
    }

    fn start_transaction(&self) -> DBIO<Transaction> {
        DBIO::new(async move { Transaction::new(self).await })
    }

    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()> {
        metrics::query("postgres.raw_cmd", cmd, &[], move || {
            async move {
                let client = self.client.lock().await;
                client.simple_query(cmd).await?;

                Ok(())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connector::Queryable, Quaint};
    use std::env;
    use url::Url;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref CONN_STR: String = env::var("TEST_PSQL").unwrap();
    }

    #[test]
    fn should_parse_socket_url() {
        let url = PostgresUrl::new(Url::parse("postgresql:///dbname?host=/var/run/psql.sock").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!("/var/run/psql.sock", url.host());
    }

    #[test]
    fn should_parse_default_host() {
        let url = PostgresUrl::new(Url::parse("postgresql:///dbname").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!("localhost", url.host());
    }

    #[tokio::test]
    async fn should_provide_a_database_connection() {
        let pool = Quaint::new(&CONN_STR).unwrap();
        let connection = pool.check_out().await.unwrap();

        let res = connection
            .query_raw(
                "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
                &[],
            )
            .await
            .unwrap();

        // No results expected.
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn should_provide_a_database_transaction() {
        let pool = Quaint::new(&CONN_STR).unwrap();
        let connection = pool.check_out().await.unwrap();
        let tx = connection.start_transaction().await.unwrap();

        let res = tx
            .query_raw(
                "select * from \"pg_catalog\".\"pg_am\" where amtype = 'x'",
                &[],
            )
            .await
            .unwrap();

        assert!(res.is_empty());
    }

    #[allow(unused)]
    const TABLE_DEF: &str = r#"
    CREATE TABLE "user"(
        id       int4    PRIMARY KEY     NOT NULL,
        name     text    NOT NULL,
        age      int4    NOT NULL,
        salary   float4
    );
    "#;

    #[allow(unused)]
    const CREATE_USER: &str = r#"
    INSERT INTO "user" (id, name, age, salary)
    VALUES (1, 'Joe', 27, 20000.00 );
    "#;

    #[allow(unused)]
    const DROP_TABLE: &str = "DROP TABLE IF EXISTS \"user\";";

    #[tokio::test]
    async fn should_map_columns_correctly() {
        let pool = Quaint::new(&CONN_STR).unwrap();
        let connection = pool.check_out().await.unwrap();

        connection.query_raw(DROP_TABLE, &[]).await.unwrap();
        connection.query_raw(TABLE_DEF, &[]).await.unwrap();
        connection.query_raw(CREATE_USER, &[]).await.unwrap();

        let rows = connection
            .query_raw("SELECT * FROM \"user\"", &[])
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["id"].as_i64(), Some(1));
        assert_eq!(row["name"].as_str(), Some("Joe"));
        assert_eq!(row["age"].as_i64(), Some(27));

        assert_eq!(row["salary"].as_f64(), Some(20000.0));
    }

    #[tokio::test]
    async fn test_custom_search_path() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.query_pairs_mut().append_pair("schema", "musti-test");

        let pool = Quaint::new(url.as_str()).unwrap();
        let client = pool.check_out().await.unwrap();

        let result_set = client.query_raw("SHOW search_path", &[]).await.unwrap();
        let row = result_set.first().unwrap();

        assert_eq!(Some("\"musti-test\""), row[0].as_str());
    }

    #[tokio::test]
    async fn should_map_nonexisting_database_error() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.set_path("this_does_not_exist");

        let pool = Quaint::new(url.as_str()).unwrap();
        let res = pool.check_out().await;

        assert!(res.is_err());

        match res {
            Ok(_) => unreachable!(),
            Err(Error::DatabaseDoesNotExist { db_name }) => {
                assert_eq!("this_does_not_exist", db_name.as_str())
            }
            Err(e) => panic!("Expected `DatabaseDoesNotExist`, got {:?}", e),
        }
    }

    #[test]
    fn postgres_params_from_url_should_capture_database_name() {
        let url: Url = "postgresql://postgres:prisma@127.0.0.1:5432/pgress?schema=test_schema"
            .parse()
            .unwrap();
        let params = PostgresParams::try_from(url).unwrap();
        assert_eq!(params.dbname, "pgress");
        assert_eq!(params.schema, "test_schema");
    }
}
