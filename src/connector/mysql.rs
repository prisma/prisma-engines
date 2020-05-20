mod conversion;
mod error;

use async_trait::async_trait;
use mysql_async::{self as my, prelude::Queryable as _, Conn};
use percent_encoding::percent_decode;
use std::{borrow::Cow, future::Future, path::Path, time::Duration};
use tokio::time::timeout;
use url::Url;

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};

/// A connector interface for the MySQL database.
#[derive(Debug)]
pub struct Mysql {
    pub(crate) pool: my::Pool,
    pub(crate) url: MysqlUrl,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
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

    pub(crate) fn connect_timeout(&self) -> Option<Duration> {
        self.query_params.connect_timeout
    }

    fn parse_query_params(url: &Url) -> Result<MysqlUrlQueryParams, Error> {
        let mut connection_limit = None;
        let mut ssl_opts = my::SslOpts::default();
        let mut use_ssl = false;
        let mut socket = None;
        let mut socket_timeout = None;
        let mut connect_timeout = None;

        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "connection_limit" => {
                    let as_int: usize = v
                        .parse()
                        .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                    connection_limit = Some(as_int);
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
                    connect_timeout = Some(Duration::from_secs(as_int));
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
    pub(crate) fn connection_limit(&self) -> Option<usize> {
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
    connection_limit: Option<usize>,
    use_ssl: bool,
    socket: Option<String>,
    socket_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
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

    async fn get_conn(&self) -> crate::Result<Conn> {
        match self.connect_timeout {
            Some(duration) => Ok(timeout(duration, self.pool.get_conn()).await??),
            None => Ok(self.pool.get_conn().await?),
        }
    }
}

impl TransactionCapable for Mysql {}

#[async_trait]
impl Queryable for Mysql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Mysql::build(q);
        self.query_raw(&sql, &params).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Mysql::build(q);
        self.execute_raw(&sql, &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("mysql.query_raw", sql, params, move || async move {
            let conn = self.get_conn().await?;
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
        })
        .await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("mysql.execute_raw", sql, params, move || async move {
            let conn = self.get_conn().await?;
            let results = self
                .timeout(conn.prep_exec(sql, conversion::conv_params(params)))
                .await?;
            Ok(results.affected_rows())
        })
        .await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("mysql.raw_cmd", cmd, &[], move || async move {
            let conn = self.get_conn().await?;
            self.timeout(conn.query(cmd)).await?;

            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT @@GLOBAL.version version"#;
        let rows = self.query_raw(query, &[]).await?;

        let version_string = rows
            .get(0)
            .and_then(|row| row.get("version").and_then(|version| version.to_string()));

        Ok(version_string)
    }
}

#[cfg(test)]
mod tests {
    use super::MysqlUrl;
    use crate::{ast::*, col, connector::Queryable, error::*, single::Quaint, val, values};
    use chrono::Utc;
    use once_cell::sync::Lazy;
    use std::env;
    use url::Url;

    static CONN_STR: Lazy<String> = Lazy::new(|| env::var("TEST_MYSQL").expect("TEST_MYSQL env var"));

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

        let ch_ch_ch_ch_changees = connection.execute_raw(CREATE_USER, &[]).await.unwrap();
        assert_eq!(1, ch_ch_ch_ch_changees);

        let rows = connection.query_raw("SELECT * FROM `user`", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["id"].as_i64(), Some(1));
        assert_eq!(row["name"].as_str(), Some("Joe"));
        assert!(row["name"].is_text());
        assert_eq!(row["age"].as_i64(), Some(27));
        assert_eq!(row["salary"].as_f64(), Some(20000.0));
    }

    #[tokio::test]
    async fn tuples_in_selection() {
        let table = r#"
            CREATE TABLE tuples (id SERIAL PRIMARY KEY, age INTEGER NOT NULL, length REAL NOT NULL);
        "#;

        let connection = Quaint::new(&CONN_STR).await.unwrap();

        connection.query_raw("DROP TABLE IF EXISTS tuples", &[]).await.unwrap();
        connection.query_raw(table, &[]).await.unwrap();

        let insert = Insert::multi_into("tuples", vec!["age", "length"])
            .values(vec![val!(35), val!(20.0)])
            .values(vec![val!(40), val!(18.0)]);

        connection.insert(insert.into()).await.unwrap();

        // 1-tuple
        {
            let mut cols = Row::new();
            cols.push(Column::from("age"));

            let mut vals = Row::new();
            vals.push(35);

            let select = Select::from_table("tuples").so_that(cols.in_selection(vals));
            let rows = connection.select(select).await.unwrap();

            let row = rows.get(0).unwrap();
            assert_eq!(row["age"].as_i64(), Some(35));
            assert_eq!(row["length"].as_f64(), Some(20.0));
        }

        // 2-tuple
        {
            let cols = Row::from((col!("age"), col!("length")));
            let vals = values!((35, 20.0));

            let select = Select::from_table("tuples").so_that(cols.in_selection(vals));
            let rows = connection.select(select).await.unwrap();

            let row = rows.get(0).unwrap();
            assert_eq!(row["age"].as_i64(), Some(35));
            assert_eq!(row["length"].as_f64(), Some(20.0));
        }
    }

    #[tokio::test]
    async fn blobs_roundtrip() {
        let connection = Quaint::new(&CONN_STR).await.unwrap();
        let blob: Vec<u8> = vec![4, 2, 0];

        connection
            .query_raw("DROP TABLE IF EXISTS mysql_blobs_roundtrip_test", &[])
            .await
            .unwrap();

        connection
            .query_raw(
                "CREATE TABLE mysql_blobs_roundtrip_test (id int AUTO_INCREMENT PRIMARY KEY, bytes MEDIUMBLOB)",
                &[],
            )
            .await
            .unwrap();

        let insert = Insert::single_into("mysql_blobs_roundtrip_test").value("bytes", blob.as_slice());

        connection.query(insert.into()).await.unwrap();

        let roundtripped = Select::from_table("mysql_blobs_roundtrip_test").column("bytes");
        let roundtripped = connection.query(roundtripped.into()).await.unwrap();

        assert_eq!(
            roundtripped.into_single().unwrap().at(0).unwrap(),
            &Value::Bytes(blob.as_slice().into())
        );
    }

    #[tokio::test]
    async fn test_mysql_time() {
        let connection = Quaint::new(&CONN_STR).await.unwrap();
        let time = chrono::NaiveTime::from_hms_micro(14, 40, 22, 1);

        connection
            .query_raw("DROP TABLE IF EXISTS quaint_mysql_time_test", &[])
            .await
            .unwrap();

        connection
            .query_raw(
                "CREATE TABLE quaint_mysql_time_test (id INTEGER AUTO_INCREMENT PRIMARY KEY, value TIME)",
                &[],
            )
            .await
            .unwrap();

        let insert_raw = "INSERT INTO quaint_mysql_time_test (value) VALUES ('20:12:22')";
        let insert_parameterized = Insert::single_into("quaint_mysql_time_test").value("value", time);

        connection.query_raw(insert_raw, &[]).await.unwrap();
        connection.query(insert_parameterized.into()).await.unwrap();

        let select = Select::from_table("quaint_mysql_time_test").value(asterisk());
        let rows = connection.query(select.into()).await.unwrap();

        assert_eq!(rows.len(), 2);

        assert_eq!(
            rows.get(0).unwrap().at(1),
            Some(&Value::DateTime("1970-01-01T20:12:22Z".parse().unwrap()))
        );
        assert_eq!(
            rows.get(1).unwrap().at(1),
            Some(&Value::DateTime("1970-01-01T14:40:22Z".parse().unwrap()))
        );
    }

    #[tokio::test]
    async fn test_mysql_datetime() {
        let connection = Quaint::new(&CONN_STR).await.unwrap();
        let datetime: chrono::DateTime<Utc> = "2003-03-01T13:10:35.789Z".parse().unwrap();

        connection
            .query_raw("DROP TABLE IF EXISTS quaint_mysql_datetime_test", &[])
            .await
            .unwrap();

        connection
            .query_raw(
                "CREATE TABLE quaint_mysql_datetime_test (id INTEGER AUTO_INCREMENT PRIMARY KEY, value DATETIME(3))",
                &[],
            )
            .await
            .unwrap();

        let insert_raw = "INSERT INTO quaint_mysql_datetime_test (value) VALUES ('2020-03-15T20:12:22.003')";
        let insert_parameterized = Insert::single_into("quaint_mysql_datetime_test").value("value", datetime);

        connection.query_raw(insert_raw, &[]).await.unwrap();
        connection.query(insert_parameterized.into()).await.unwrap();

        let select = Select::from_table("quaint_mysql_datetime_test").value(asterisk());
        let rows = connection.query(select.into()).await.unwrap();

        assert_eq!(rows.len(), 2);

        assert_eq!(
            rows.get(0).unwrap().at(1),
            Some(&Value::DateTime("2020-03-15T20:12:22.003Z".parse().unwrap()))
        );
        assert_eq!(
            rows.get(1).unwrap().at(1),
            Some(&Value::DateTime("2003-03-01T13:10:35.789Z".parse().unwrap()))
        );
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

        // Error code 1364
        {
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

        // Error code 1048
        {
            conn.query_raw(
                "INSERT INTO test_null_constraint_violation (id1, id2) VALUES (50, 55)",
                &[],
            )
            .await
            .unwrap();

            let err = conn
                .query_raw("UPDATE test_null_constraint_violation SET id2 = NULL", &[])
                .await
                .unwrap_err();

            match err.kind() {
                ErrorKind::NullConstraintViolation { constraint } => {
                    assert_eq!(Some("1048"), err.original_code());
                    assert_eq!(&DatabaseConstraint::Fields(vec![String::from("id2")]), constraint);
                }
                _ => panic!("{:?}", err),
            }
        }
    }

    #[tokio::test]
    async fn text_columns_with_non_utf8_encodings_can_be_queried() {
        use crate::ast;

        let conn = Quaint::new(&CONN_STR).await.unwrap();

        conn.execute_raw("DROP TABLE IF EXISTS `encodings_test`", &[])
            .await
            .unwrap();

        let create_table = r#"
            CREATE TABLE `encodings_test` (
                id INTEGER AUTO_INCREMENT PRIMARY KEY,
                gb18030 VARCHAR(100) CHARACTER SET gb18030
            );
        "#;

        conn.execute_raw(create_table, &[]).await.unwrap();

        let insert = r#"
            INSERT INTO `encodings_test` (gb18030)
            VALUES ("法式咸派"), (?)
        "#;

        conn.query_raw(insert, &["土豆".into()]).await.unwrap();

        let select = ast::Select::from_table("encodings_test").value(ast::asterisk());
        let result = conn.query(select.into()).await.unwrap();

        assert_eq!(
            result.get(0).unwrap().get("gb18030").unwrap(),
            &Value::Text("法式咸派".into())
        );

        assert_eq!(
            result.get(1).unwrap().get("gb18030").unwrap(),
            &Value::Text("土豆".into())
        );
    }

    #[tokio::test]
    async fn filtering_by_json_values_does_not_work_but_does_not_crash() {
        let create_table = r#"
            CREATE TABLE `nested` (
                id       int4 AUTO_INCREMENT PRIMARY KEY,
                nested   json NOT NULL
            );
        "#;

        let drop_table = "DROP TABLE IF EXISTS `nested`";

        let conn = Quaint::new(&CONN_STR).await.unwrap();

        conn.query_raw(drop_table, &[]).await.unwrap();
        conn.query_raw(create_table, &[]).await.unwrap();

        let insert = Insert::multi_into("nested", &["nested"])
            .values(vec!["{\"isTrue\": true}"])
            .values(vec!["{\"isTrue\": false}"]);

        conn.query(insert.into()).await.unwrap();

        let select = Select::from_table("nested").so_that("nested".equals("{\"isTrue\": false}"));

        let result = conn.query(select.into()).await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn mysql_float_columns_cast_to_f32() {
        let create_table = r#"
            CREATE TABLE `float_precision_test` (
                id int4 AUTO_INCREMENT PRIMARY KEY,
                f  float NOT NULL
            )
        "#;

        let drop_table = "DROP TABLE IF EXISTS `float_precision_test`";

        let conn = Quaint::new(&CONN_STR).await.unwrap();

        conn.query_raw(drop_table, &[]).await.unwrap();
        conn.query_raw(create_table, &[]).await.unwrap();

        let insert = Insert::single_into("float_precision_test").value("f", 6.4123456);
        let select = Select::from_table("float_precision_test").column("f");

        conn.query(insert.into()).await.unwrap();
        let result = conn.query(select.into()).await.unwrap();

        assert_eq!(result.into_single().unwrap().at(0).unwrap().as_f64().unwrap(), 6.412345);
    }

    #[tokio::test]
    async fn newdecimal_conversion_is_handled_correctly() {
        let conn = Quaint::new(&CONN_STR).await.unwrap();
        let result = conn.query_raw("SELECT SUM(1) AS THEONE", &[]).await.unwrap();

        assert_eq!(result.into_single().unwrap()[0], Value::Real("1.0".parse().unwrap()));
    }

    #[tokio::test]
    async fn json_conversion_is_handled_correctly() {
        let create_table = r#"
            CREATE TABLE `json_test` (
                id int4 AUTO_INCREMENT PRIMARY KEY,
                j  json NOT NULL
            )
        "#;

        let drop_table = "DROP TABLE IF EXISTS `json_test`";
        let conn = Quaint::new(&CONN_STR).await.unwrap();

        conn.query_raw(drop_table, &[]).await.unwrap();
        conn.query_raw(create_table, &[]).await.unwrap();

        let insert = Insert::single_into("json_test").value("j", r#"{"some": "json"}"#);
        let select = Select::from_table("json_test").column("j");

        conn.query(insert.into()).await.unwrap();
        let result = conn.query(select.into()).await.unwrap();

        assert_eq!(
            result
                .into_single()
                .unwrap()
                .at(0)
                .unwrap()
                .as_json()
                .unwrap()
                .to_string(),
            r#"{"some":"json"}"#
        );
    }

    #[tokio::test]
    async fn unsigned_integers_are_handled() {
        let conn = Quaint::new(&CONN_STR).await.unwrap();

        let create_table = r#"
            CREATE TABLE `unsigned_integers_test` (
                id int4 AUTO_INCREMENT PRIMARY KEY,
                big BIGINT UNSIGNED
            )
        "#;

        let drop_table = r#"DROP TABLE IF EXISTS `unsigned_integers_test`"#;

        conn.query_raw(drop_table, &[]).await.unwrap();
        conn.query_raw(create_table, &[]).await.unwrap();

        let insert = Insert::multi_into("unsigned_integers_test", &["big"])
            .values((2,))
            .values((std::i64::MAX,));
        conn.query(insert.into()).await.unwrap();

        let select_star = Select::from_table("unsigned_integers_test")
            .column("big")
            .order_by(Column::from("id"));
        let roundtripped = conn.query(select_star.into()).await.unwrap();

        let expected = &[2, std::i64::MAX];
        let actual: Vec<i64> = roundtripped
            .into_iter()
            .map(|row| row.at(0).unwrap().as_i64().unwrap())
            .collect();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn integer_out_of_range_errors() {
        let conn = Quaint::new(&CONN_STR).await.unwrap();

        let create_table = r#"
            CREATE TABLE `out_of_range_integers_test` (
                id int4 AUTO_INCREMENT PRIMARY KEY,
                big INT4 UNSIGNED
            )
        "#;

        let drop_table = r#"DROP TABLE IF EXISTS `out_of_range_integers_test`"#;

        conn.query_raw(drop_table, &[]).await.unwrap();
        conn.query_raw(create_table, &[]).await.unwrap();

        // Negative value
        {
            let insert = Insert::multi_into("out_of_range_integers_test", &["big"]).values((-22,));
            let result = conn.query(insert.into()).await;

            assert!(matches!(result.unwrap_err().kind(), ErrorKind::ValueOutOfRange { .. }));
        }

        // Value too big
        {
            let insert = Insert::multi_into("out_of_range_integers_test", &["big"]).values((std::i64::MAX,));
            let result = conn.query(insert.into()).await;

            assert!(matches!(result.unwrap_err().kind(), ErrorKind::ValueOutOfRange { .. }));
        }
    }

    #[tokio::test]
    async fn bigint_unsigned_can_overflow() {
        let conn = Quaint::new(&CONN_STR).await.unwrap();

        let create_table = r#"
            CREATE TABLE `bigint_unsigned_test` (
                id int4 AUTO_INCREMENT PRIMARY KEY,
                big BIGINT UNSIGNED
            )
        "#;

        let drop_table = r#"DROP TABLE IF EXISTS `bigint_unsigned_test`"#;

        let insert = r#"INSERT INTO `bigint_unsigned_test` (`big`) VALUES (18446744073709551615)"#;

        conn.query_raw(drop_table, &[]).await.unwrap();
        conn.query_raw(create_table, &[]).await.unwrap();
        conn.query_raw(insert, &[]).await.unwrap();

        let result = conn.query_raw("SELECT * FROM `bigint_unsigned_test`", &[]).await;

        assert!(
            matches!(result.unwrap_err().kind(), ErrorKind::ValueOutOfRange { message } if message == "Unsigned integers larger than 9_223_372_036_854_775_807 are currently not handled.")
        );
    }

    #[tokio::test]
    async fn json_filtering_works() {
        let conn = Quaint::new(&CONN_STR).await.unwrap();

        let create_table = r#"
            CREATE TABLE `table_with_json` (
                id int4 AUTO_INCREMENT PRIMARY KEY,
                obj json
            )
        "#;

        let drop_table = r#"DROP TABLE IF EXISTS `table_with_json`"#;

        let insert = Insert::single_into("table_with_json").value("obj", serde_json::json!({ "a": "a" }));
        let second_insert = Insert::single_into("table_with_json").value("obj", serde_json::json!({ "a": "b" }));

        conn.query_raw(drop_table, &[]).await.unwrap();
        conn.query_raw(create_table, &[]).await.unwrap();
        conn.query(insert.into()).await.unwrap();
        conn.query(second_insert.into()).await.unwrap();

        // Equals
        {
            let select = Select::from_table("table_with_json")
                .value(asterisk())
                .so_that(Column::from("obj").equals(Value::Json(serde_json::json!({ "a": "b" }))));

            let result = conn.query(select.into()).await.unwrap();

            assert_eq!(result.len(), 1);
            assert_eq!(result.get(0).unwrap().get("id").unwrap(), &Value::Integer(2))
        }

        // Not equals
        {
            let select = Select::from_table("table_with_json")
                .value(asterisk())
                .so_that(Column::from("obj").not_equals(Value::Json(serde_json::json!({ "a": "a" }))));

            let result = conn.query(select.into()).await.unwrap();

            assert_eq!(result.len(), 1);
            assert_eq!(result.get(0).unwrap().get("id").unwrap(), &Value::Integer(2))
        }
    }
}
