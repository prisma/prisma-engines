mod conversion;
mod error;

use mysql_async::{self as my, prelude::Queryable as _};
use percent_encoding::percent_decode;
use std::{borrow::Cow, future::Future, path::Path, time::Duration};
use tokio::time::timeout;
use url::Url;

use crate::{
    ast::{ParameterizedValue, Query},
    connector::{metrics, queryable::*, ResultSet, DBIO},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};

/// A connector interface for the MySQL database.
#[derive(Debug)]
pub struct Mysql {
    pub(crate) pool: my::Pool,
    pub(crate) url: MysqlUrl,
    socket_timeout: Option<Duration>,
    connect_timeout: Duration,
}

/// Wraps a connection url and exposes the parsing logic used by quaint, including default values.
#[derive(Debug, Clone)]
pub struct MysqlUrl {
    url: Url,
    query_params: MysqlUrlQueryParams,
}

impl MysqlUrl {
    /// Parse `Url` to `MysqlUrl`. Returns error for mistyped connection
    /// parameters.
    pub fn new(url: Url) -> Result<Self, Error> {
        let query_params = Self::parse_query_params(&url)?;

        Ok(Self { url, query_params })
    }

    /// The bare `Url` to the database.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// The percent-decoded database username.
    pub fn username(&self) -> Cow<str> {
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

    /// The percent-decoded database password.
    pub fn password(&self) -> Option<Cow<str>> {
        match self
            .url
            .password()
            .and_then(|pw| percent_decode(pw.as_bytes()).decode_utf8().ok())
        {
            Some(password) => Some(password),
            None => self.url.password().map(|s| s.into()),
        }
    }

    /// Name of the database connected. Defaults to `mysql`.
    pub fn dbname(&self) -> &str {
        match self.url.path_segments() {
            Some(mut segments) => segments.next().unwrap_or("mysql"),
            None => "mysql",
        }
    }

    /// The database host. If `socket` and `host` are not set, defaults to `localhost`.
    pub fn host(&self) -> &str {
        self.url.host_str().unwrap_or("localhost")
    }

    /// If set, connected to the database through a Unix socket.
    pub fn socket(&self) -> &Option<String> {
        &self.query_params.socket
    }

    /// The database port, defaults to `3306`.
    pub fn port(&self) -> u16 {
        self.url.port().unwrap_or(3306)
    }

    fn default_connection_limit() -> usize {
        num_cpus::get_physical() * 2 + 1
    }

    fn parse_query_params(url: &Url) -> Result<MysqlUrlQueryParams, Error> {
        let mut connection_limit = Self::default_connection_limit();
        let mut ssl_opts = my::SslOpts::default();
        let mut use_ssl = false;
        let mut socket = None;
        let mut socket_timeout = None;
        let mut connect_timeout = Duration::from_secs(5);

        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "connection_limit" => {
                    let as_int: usize = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    connection_limit = as_int;
                }
                "sslcert" => {
                    use_ssl = true;
                    ssl_opts.set_root_cert_path(Some(Path::new(&*v).to_path_buf()));
                }
                "sslidentity" => {
                    use_ssl = true;
                    ssl_opts.set_pkcs12_path(Some(Path::new(&*v).to_path_buf()));
                }
                "sslpassword" => {
                    use_ssl = true;
                    ssl_opts.set_password(Some(v.to_string()));
                }
                "socket" => {
                    socket = Some(v.replace("(", "").replace(")", ""));
                }
                "socket_timeout" => {
                    let as_int = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                    socket_timeout = Some(Duration::from_secs(as_int));
                }
                "connect_timeout" => {
                    let as_int = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;
                    connect_timeout = Duration::from_secs(as_int);
                }
                "sslaccept" => {
                    match v.as_ref() {
                        "strict" => {}
                        "accept_invalid_certs" => {
                            ssl_opts.set_danger_accept_invalid_certs(true);
                        }
                        _ => {
                            #[cfg(not(feature = "tracing-log"))]
                            debug!("Unsupported SSL accept mode {}, defaulting to `strict`", v);
                            #[cfg(feature = "tracing-log")]
                            tracing::debug!(
                                message = "Unsupported SSL accept mode, defaulting to `strict`",
                                mode = &*v
                            );
                        }
                    };
                }
                _ => {
                    #[cfg(not(feature = "tracing-log"))]
                    trace!("Discarding connection string param: {}", k);
                    #[cfg(feature = "tracing-log")]
                    tracing::trace!(message = "Discarding connection string param", param = &*k);
                }
            };
        }

        Ok(MysqlUrlQueryParams {
            ssl_opts,
            connection_limit,
            use_ssl,
            socket,
            connect_timeout,
            socket_timeout,
        })
    }

    #[cfg(feature = "pooled")]
    pub(crate) fn connection_limit(&self) -> usize {
        self.query_params.connection_limit
    }

    pub(crate) fn to_opts_builder(&self) -> my::OptsBuilder {
        let mut config = my::OptsBuilder::new();

        config.user(Some(self.username()));
        config.pass(self.password());
        config.db_name(Some(self.dbname()));

        match self.socket() {
            Some(ref socket) => {
                config.socket(Some(socket));
            }
            None => {
                config.ip_or_hostname(self.host());
                config.tcp_port(self.port());
            }
        }

        config.stmt_cache_size(Some(1000));
        config.conn_ttl(Some(Duration::from_secs(5)));

        if self.query_params.use_ssl {
            config.ssl_opts(Some(self.query_params.ssl_opts.clone()));
        }

        config
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MysqlUrlQueryParams {
    ssl_opts: my::SslOpts,
    connection_limit: usize,
    use_ssl: bool,
    socket: Option<String>,
    socket_timeout: Option<Duration>,
    connect_timeout: Duration,
}

impl Mysql {
    /// Create a new MySQL connection using `OptsBuilder` from the `mysql` crate.
    pub fn new(url: MysqlUrl) -> crate::Result<Self> {
        let mut opts = url.to_opts_builder();
        let pool_opts = my::PoolOptions::with_constraints(my::PoolConstraints::new(1, 1).unwrap());
        opts.pool_options(pool_opts);

        Ok(Self {
            socket_timeout: url.query_params.socket_timeout,
            connect_timeout: url.query_params.connect_timeout,
            pool: my::Pool::new(opts),
            url,
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

impl TransactionCapable for Mysql {}

impl Queryable for Mysql {
    fn query<'a>(&'a self, q: Query<'a>) -> DBIO<'a, ResultSet> {
        DBIO::new(async move {
            let (sql, params) = visitor::Mysql::build(q);
            self.query_raw(&sql, &params).await
        })
    }

    fn query_raw<'a>(&'a self, sql: &'a str, params: &'a [ParameterizedValue]) -> DBIO<'a, ResultSet> {
        metrics::query("mysql.query_raw", sql, params, move || {
            async move {
                let conn = timeout(self.connect_timeout, self.pool.get_conn()).await??;
                let results = self
                    .timeout(conn.prep_exec(sql, conversion::conv_params(params)))
                    .await?;

                let columns = results
                    .columns_ref()
                    .iter()
                    .map(|s| s.name_str().into_owned())
                    .collect();

                let last_id = results.last_insert_id();
                let mut result_set = ResultSet::new(columns, Vec::new());

                let (_, rows) = self
                    .timeout(results.map_and_drop(|mut row| row.take_result_row()))
                    .await?;

                for row in rows.into_iter() {
                    result_set.rows.push(row?);
                }

                if let Some(id) = last_id {
                    result_set.set_last_insert_id(id);
                };

                Ok(result_set)
            }
        })
    }

    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()> {
        metrics::query("mysql.raw_cmd", cmd, &[], move || {
            async move {
                let conn = timeout(self.connect_timeout, self.pool.get_conn()).await??;
                self.timeout(conn.query(cmd)).await?;

                Ok(())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::MysqlUrl;
    use crate::{connector::Queryable, error::*, single::Quaint};
    use lazy_static::lazy_static;
    use std::env;
    use url::Url;

    lazy_static! {
        static ref CONN_STR: String = env::var("TEST_MYSQL").unwrap();
    }

    #[test]
    fn should_parse_socket_url() {
        let url = MysqlUrl::new(Url::parse("mysql://root@localhost/dbname?socket=(/tmp/mysql.sock)").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!(&Some(String::from("/tmp/mysql.sock")), url.socket());
    }

    #[tokio::test]
    async fn should_provide_a_database_connection() {
        let connection = Quaint::new(&CONN_STR).await.unwrap();

        let res = connection
            .query_raw(
                "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
                &[],
            )
            .await
            .unwrap();

        assert!(res.is_empty());
    }

    const TABLE_DEF: &str = r#"
CREATE TABLE `user`(
    id       int4    PRIMARY KEY     NOT NULL,
    name     text    NOT NULL,
    age      int4    NOT NULL,
    salary   float4
);
"#;

    const CREATE_USER: &str = r#"
INSERT INTO `user` (id, name, age, salary)
VALUES (1, 'Joe', 27, 20000.00 );
"#;

    const DROP_TABLE: &str = "DROP TABLE IF EXISTS `user`;";

    #[tokio::test]
    async fn should_map_columns_correctly() {
        let connection = Quaint::new(&CONN_STR).await.unwrap();

        connection.query_raw(DROP_TABLE, &[]).await.unwrap();
        connection.query_raw(TABLE_DEF, &[]).await.unwrap();
        connection.query_raw(CREATE_USER, &[]).await.unwrap();

        let rows = connection.query_raw("SELECT * FROM `user`", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["id"].as_i64(), Some(1));
        assert_eq!(row["name"].as_str(), Some("Joe"));
        assert_eq!(row["age"].as_i64(), Some(27));
        assert_eq!(row["salary"].as_f64(), Some(20000.0));
    }

    #[tokio::test]
    async fn should_map_nonexisting_database_error() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.set_username("root").unwrap();
        url.set_path("/this_does_not_exist");

        let url = url.as_str().to_string();
        let conn = Quaint::new(&url).await.unwrap();
        let res = conn.query_raw("SELECT 1 + 1", &[]).await;

        assert!(&res.is_err());

        let err = res.unwrap_err();

        match err.kind() {
            ErrorKind::DatabaseDoesNotExist { db_name } => {
                assert_eq!(Some("1049"), err.original_code());
                assert_eq!(Some("Unknown database \'this_does_not_exist\'"), err.original_message());
                assert_eq!("this_does_not_exist", db_name.as_str())
            }
            e => panic!("Expected `DatabaseDoesNotExist`, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_uniq_constraint_violation() {
        let conn = Quaint::new(&CONN_STR).await.unwrap();

        let _ = conn.raw_cmd("DROP TABLE test_uniq_constraint_violation").await;
        let _ = conn.raw_cmd("DROP INDEX idx_uniq_constraint_violation").await;

        conn.raw_cmd("CREATE TABLE test_uniq_constraint_violation (id1 int, id2 int)")
            .await
            .unwrap();
        conn.raw_cmd("CREATE UNIQUE INDEX idx_uniq_constraint_violation ON test_uniq_constraint_violation (id1, id2) USING btree").await.unwrap();

        conn.query_raw(
            "INSERT INTO test_uniq_constraint_violation (id1, id2) VALUES (1, 2)",
            &[],
        )
        .await
        .unwrap();

        let res = conn
            .query_raw(
                "INSERT INTO test_uniq_constraint_violation (id1, id2) VALUES (1, 2)",
                &[],
            )
            .await;

        let err = res.unwrap_err();

        match err.kind() {
            ErrorKind::UniqueConstraintViolation { constraint } => {
                assert_eq!(Some("1062"), err.original_code());
                assert_eq!(
                    &DatabaseConstraint::Index(String::from("idx_uniq_constraint_violation")),
                    constraint,
                )
            }
            _ => panic!(err),
        }
    }

    #[tokio::test]
    async fn test_null_constraint_violation() {
        let conn = Quaint::new(&CONN_STR).await.unwrap();

        let _ = conn.raw_cmd("DROP TABLE test_null_constraint_violation").await;

        conn.raw_cmd("CREATE TABLE test_null_constraint_violation (id1 int not null, id2 int not null)")
            .await
            .unwrap();

        let res = conn
            .query_raw("INSERT INTO test_null_constraint_violation () VALUES ()", &[])
            .await;

        let err = res.unwrap_err();

        match err.kind() {
            ErrorKind::NullConstraintViolation { constraint } => {
                assert_eq!(Some("1364"), err.original_code());
                assert_eq!(
                    Some("Field \'id1\' doesn\'t have a default value"),
                    err.original_message()
                );
                assert_eq!(&DatabaseConstraint::Fields(vec![String::from("id1")]), constraint)
            }
            _ => panic!(err),
        }
    }
}
