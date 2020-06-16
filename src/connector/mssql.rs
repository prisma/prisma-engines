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
            let params = conversion::conv_params(params);
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
            let params = conversion::conv_params(params);
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
                                let kind = ErrorKind::ConversionError("Malformed connection string key");
                                Error::builder(kind).build()
                            })?
                            .trim();

                        let value = split.next().ok_or_else(|| {
                            let kind = ErrorKind::ConversionError("Malformed connection string value");
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
                let kind = ErrorKind::ConversionError("Malformed connection string");
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
    use crate::{ast::*, pooled, prelude::*, single, val};
    use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
    use names::Generator;
    use once_cell::sync::Lazy;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::env;
    use uuid::Uuid;

    static CONN_STR: Lazy<String> = Lazy::new(|| env::var("TEST_MSSQL").expect("TEST_MSSQL env var"));

    fn random_table() -> String {
        let mut generator = Generator::default();
        let name = generator.next().unwrap().replace('-', "");
        format!("##{}", name)
    }

    #[tokio::test]
    async fn database_connection() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;

        let res = connection.query_raw("SELECT 1", &[]).await?;
        let row = res.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn pooled_database_connection() -> crate::Result<()> {
        let pool = pooled::Quaint::builder(&CONN_STR)?.build();
        let connection = pool.check_out().await?;

        let res = connection.query_raw("SELECT 1", &[]).await?;
        let row = res.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn transactions() -> crate::Result<()> {
        let pool = pooled::Quaint::builder(&CONN_STR)?.build();
        let connection = pool.check_out().await?;

        let tx = connection.start_transaction().await?;
        let res = tx.query_raw("SELECT 1", &[]).await?;

        tx.commit().await?;

        let row = res.get(0).unwrap();

        assert_eq!(row[0].as_i64(), Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn aliased_value() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let query = Select::default().value(val!(1).alias("test"));

        let res = connection.select(query).await?;
        let row = res.get(0).unwrap();

        // No results expected.
        assert_eq!(row["test"].as_i64(), Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn aliased_null() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let query = Select::default().value(val!(Value::Integer(None)).alias("test"));

        let res = connection.select(query).await?;
        let row = res.get(0).unwrap();

        // No results expected.
        assert!(row["test"].is_null());

        Ok(())
    }

    #[tokio::test]
    async fn select_star_from() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id int, id2 int)", table))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id, id2) VALUES (1, 2)", table))
            .await?;

        let query = Select::from_table(table);
        let res = connection.select(query).await?;
        let row = res.get(0).unwrap();

        assert_eq!(row["id"].as_i64(), Some(1));
        assert_eq!(row["id2"].as_i64(), Some(2));

        Ok(())
    }

    #[tokio::test]
    async fn in_values_tuple() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id int, id2 int)", table))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id, id2) VALUES (1,2),(3,4),(5,6)", table))
            .await?;

        let query = Select::from_table(table)
            .so_that(Row::from((col!("id"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

        let res = connection.select(query).await?;
        assert_eq!(2, res.len());

        let row1 = res.get(0).unwrap();
        assert_eq!(Some(1), row1["id"].as_i64());
        assert_eq!(Some(2), row1["id2"].as_i64());

        let row2 = res.get(1).unwrap();
        assert_eq!(Some(3), row2["id"].as_i64());
        assert_eq!(Some(4), row2["id2"].as_i64());

        Ok(())
    }

    #[tokio::test]
    async fn not_in_values_tuple() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id int, id2 int)", table))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id, id2) VALUES (1,2),(3,4),(5,6)", table))
            .await?;

        let query = Select::from_table(table)
            .so_that(Row::from((col!("id"), col!("id2"))).not_in_selection(values!((1, 2), (3, 4))));

        let res = connection.select(query).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(5), row["id"].as_i64());
        assert_eq!(Some(6), row["id2"].as_i64());

        Ok(())
    }

    #[tokio::test]
    async fn in_values_singular() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id int, id2 int)", table))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id, id2) VALUES (1,2),(3,4),(5,6)", table))
            .await?;

        let query = Select::from_table(table).so_that("id".in_selection(vec![1, 3]));

        let res = connection.select(query).await?;
        assert_eq!(2, res.len());

        let row1 = res.get(0).unwrap();
        assert_eq!(Some(1), row1["id"].as_i64());
        assert_eq!(Some(2), row1["id2"].as_i64());

        let row2 = res.get(1).unwrap();
        assert_eq!(Some(3), row2["id"].as_i64());
        assert_eq!(Some(4), row2["id2"].as_i64());

        Ok(())
    }

    #[tokio::test]
    async fn order_by_ascend() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id int, id2 int)", table))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id, id2) VALUES (3,4),(1,2),(5,6)", table))
            .await?;

        let query = Select::from_table(table).order_by("id2".ascend());

        let res = connection.select(query).await?;
        assert_eq!(3, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some(2), row["id2"].as_i64());

        let row = res.get(1).unwrap();
        assert_eq!(Some(3), row["id"].as_i64());
        assert_eq!(Some(4), row["id2"].as_i64());

        let row = res.get(2).unwrap();
        assert_eq!(Some(5), row["id"].as_i64());
        assert_eq!(Some(6), row["id2"].as_i64());

        Ok(())
    }

    #[tokio::test]
    async fn order_by_descend() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id int, id2 int)", table))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id, id2) VALUES (3,4),(1,2),(5,6)", table))
            .await?;

        let query = Select::from_table(table).order_by("id2".descend());

        let res = connection.select(query).await?;
        assert_eq!(3, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(5), row["id"].as_i64());
        assert_eq!(Some(6), row["id2"].as_i64());

        let row = res.get(1).unwrap();
        assert_eq!(Some(3), row["id"].as_i64());
        assert_eq!(Some(4), row["id2"].as_i64());

        let row = res.get(2).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some(2), row["id2"].as_i64());

        Ok(())
    }

    #[tokio::test]
    async fn fields_from() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Naukio')",
                table
            ))
            .await?;

        let query = Select::from_table(table).column("name").order_by("id");
        let res = connection.select(query).await?;

        assert_eq!(2, res.len());
        assert_eq!(1, res.columns().len());

        let row = res.get(0).unwrap();
        assert_eq!(Some("Musti"), row["name"].as_str());

        let row = res.get(1).unwrap();
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn where_equals() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Naukio')",
                table
            ))
            .await?;

        let query = Select::from_table(table).so_that("name".equals("Naukio"));
        let res = connection.select(query).await?;

        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn where_like() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Naukio')",
                table
            ))
            .await?;

        let query = Select::from_table(table).so_that("name".like("auk"));
        let res = connection.select(query).await?;

        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn where_not_like() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Naukio')",
                table
            ))
            .await?;

        let query = Select::from_table(table).so_that("name".not_like("auk"));
        let res = connection.select(query).await?;

        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some("Musti"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn inner_join() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table1 = random_table();
        let table2 = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table1))
            .await?;

        connection
            .raw_cmd(&format!("CREATE TABLE {} (t1_id INT, is_cat bit)", table2))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Belka')",
                table1
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (t1_id,is_cat) VALUES (1,1),(2,0)", table2))
            .await?;

        let query = Select::from_table(&table1)
            .column((&table1, "name"))
            .column((&table2, "is_cat"))
            .inner_join(
                table2
                    .as_str()
                    .on((table1.as_str(), "id").equals(Column::from((&table2, "t1_id")))),
            )
            .order_by("id".ascend());

        let res = connection.select(query).await?;

        assert_eq!(2, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some("Musti"), row["name"].as_str());
        assert_eq!(Some(true), row["is_cat"].as_bool());

        let row = res.get(1).unwrap();
        assert_eq!(Some("Belka"), row["name"].as_str());
        assert_eq!(Some(false), row["is_cat"].as_bool());

        Ok(())
    }

    #[tokio::test]
    async fn left_join() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table1 = random_table();
        let table2 = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table1))
            .await?;

        connection
            .raw_cmd(&format!("CREATE TABLE {} (t1_id INT, is_cat bit)", table2))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Belka')",
                table1
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (t1_id,is_cat) VALUES (1,1)", table2))
            .await?;

        let query = Select::from_table(&table1)
            .column((&table1, "name"))
            .column((&table2, "is_cat"))
            .left_join(
                table2
                    .as_str()
                    .on((&table1, "id").equals(Column::from((&table2, "t1_id")))),
            )
            .order_by("id".ascend());

        let res = connection.select(query).await?;

        assert_eq!(2, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some("Musti"), row["name"].as_str());
        assert_eq!(Some(true), row["is_cat"].as_bool());

        let row = res.get(1).unwrap();
        assert_eq!(Some("Belka"), row["name"].as_str());
        assert_eq!(None, row["is_cat"].as_bool());

        Ok(())
    }

    #[tokio::test]
    async fn aliasing() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let query = Select::default().value(val!(1.23).alias("foo"));

        let res = connection.select(query).await?;
        let row = res.get(0).unwrap();

        assert_eq!(Some(1.23), row["foo"].as_f64());

        Ok(())
    }

    #[tokio::test]
    async fn limit_no_offset() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Naukio')",
                table
            ))
            .await?;

        let query = Select::from_table(table).order_by("id".descend()).limit(1);

        let res = connection.select(query).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();

        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn offset_no_limit() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Naukio')",
                table
            ))
            .await?;

        let query = Select::from_table(table).order_by("id".descend()).offset(1);

        let res = connection.select(query).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();

        assert_eq!(Some("Musti"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn limit_with_offset() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Naukio'),(3, 'Belka')",
                table
            ))
            .await?;

        let query = Select::from_table(table).order_by("id".ascend()).limit(1).offset(2);

        let res = connection.select(query).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();

        assert_eq!(Some("Belka"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn limit_with_offset_no_given_order() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        connection
            .raw_cmd(&format!(
                "INSERT INTO {} (id,name) VALUES (1,'Musti'),(2, 'Naukio'),(3, 'Belka')",
                table
            ))
            .await?;

        let query = Select::from_table(table).limit(1).offset(2);

        let res = connection.select(query).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some("Belka"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_default_value_insert() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT DEFAULT 1, name NVARCHAR(max) DEFAULT 'Musti')",
                table
            ))
            .await?;

        let insert = Insert::single_into(&table);
        let changes = connection.execute(insert.into()).await?;
        assert_eq!(1, changes);

        let select = Select::from_table(&table);

        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        let insert = Insert::single_into(&table).value("id", 2).value("name", "Naukio");
        let changes = connection.execute(insert.into()).await?;
        assert_eq!(1, changes);

        let select = Select::from_table(&table);

        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(2), row["id"].as_i64());
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn returning_insert() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        let insert = Insert::single_into(&table).value("id", 2).value("name", "Naukio");

        let res = connection
            .insert(Insert::from(insert).returning(vec!["id", "name"]))
            .await?;

        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(2), row["id"].as_i64());
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn multi_insert() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(max))", table))
            .await?;

        let insert = Insert::multi_into(&table, vec!["id", "name"])
            .values(vec![val!(1), val!("Musti")])
            .values(vec![val!(2), val!("Naukio")]);

        let changes = connection.execute(insert.into()).await?;
        assert_eq!(2, changes);

        let select = Select::from_table(&table);

        let res = connection.select(select).await?;
        assert_eq!(2, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        let row = res.get(1).unwrap();
        assert_eq!(Some(2), row["id"].as_i64());
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_single_unique() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT PRIMARY KEY, name NVARCHAR(max))",
                table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (1,'Musti')", table_name))
            .await?;

        let table = Table::from(&table_name).add_unique_index("id");
        let cols = vec![(&table_name, "id"), (&table_name, "name")];

        let insert: Insert<'_> = Insert::multi_into(table.clone(), cols)
            .values(vec![val!(1), val!("Naukio")])
            .values(vec![val!(2), val!("Belka")])
            .into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(1, changes);

        let select = Select::from_table(table);

        let res = connection.select(select).await?;
        assert_eq!(2, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        let row = res.get(1).unwrap();
        assert_eq!(Some(2), row["id"].as_i64());
        assert_eq!(Some("Belka"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_single_unique_with_default() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT PRIMARY KEY DEFAULT 10, name NVARCHAR(max))",
                table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (10,'Musti')", table_name))
            .await?;

        let id = Column::from("id").default(10);
        let table = Table::from(&table_name).add_unique_index(id);

        let insert: Insert<'_> = Insert::single_into(table.clone()).value("name", "Naukio").into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(0, changes);

        let select = Select::from_table(table);

        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(10), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_single_unique_with_autogen_default() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT IDENTITY(1,1) PRIMARY KEY, name NVARCHAR(max))",
                table_name,
            ))
            .await?;

        let id = Column::from("id").default(DefaultValue::Generated);
        let table = Table::from(&table_name).add_unique_index(id);

        let insert: Insert<'_> = Insert::single_into(table.clone()).value("name", "Naukio").into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(1, changes);

        let select = Select::from_table(table);

        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_with_returning() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT PRIMARY KEY, name NVARCHAR(max))",
                table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (1,'Musti')", table_name))
            .await?;

        let table = Table::from(&table_name).add_unique_index("id");
        let cols = vec![(&table_name, "id"), (&table_name, "name")];

        let insert: Insert<'_> = Insert::multi_into(table.clone(), cols)
            .values(vec![val!(1), val!("Naukio")])
            .values(vec![val!(2), val!("Belka")])
            .into();

        let res = connection
            .insert(insert.on_conflict(OnConflict::DoNothing).returning(vec!["name"]))
            .await?;

        assert_eq!(1, res.len());
        assert_eq!(1, res.columns().len());

        let row = res.get(0).unwrap();
        assert_eq!(Some("Belka"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_two_uniques() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT PRIMARY KEY, name NVARCHAR(4000) UNIQUE)",
                table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (1,'Musti')", table_name))
            .await?;

        let table = Table::from(&table_name).add_unique_index("id").add_unique_index("name");

        let cols = vec![(&table_name, "id"), (&table_name, "name")];

        let insert: Insert<'_> = Insert::multi_into(table.clone(), cols)
            .values(vec![val!(1), val!("Naukio")])
            .values(vec![val!(3), val!("Musti")])
            .values(vec![val!(2), val!("Belka")])
            .into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(1, changes);

        let select = Select::from_table(table).order_by("id".ascend());

        let res = connection.select(select).await?;
        assert_eq!(2, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        let row = res.get(1).unwrap();
        assert_eq!(Some(2), row["id"].as_i64());
        assert_eq!(Some("Belka"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_two_uniques_with_default() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT PRIMARY KEY, name NVARCHAR(4000) UNIQUE DEFAULT 'Musti')",
                table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (1,'Musti')", table_name))
            .await?;

        let id = Column::from("id").table(&table_name);
        let name = Column::from("name").default("Musti").table(&table_name);

        let table = Table::from(&table_name)
            .add_unique_index(id.clone())
            .add_unique_index(name.clone());

        let insert: Insert<'_> = Insert::single_into(table.clone()).value(id, 2).into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(0, changes);

        let select = Select::from_table(table).order_by("id".ascend());

        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_compoud_unique() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();
        let index_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(4000))", table_name,))
            .await?;

        connection
            .raw_cmd(&format!(
                "CREATE UNIQUE INDEX {} ON {} (id ASC, name ASC)",
                index_name, table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (1,'Musti')", table_name))
            .await?;

        let id = Column::from("id").table(&table_name);
        let name = Column::from("name").table(&table_name);

        let table = Table::from(&table_name).add_unique_index(vec![id.clone(), name.clone()]);

        let insert: Insert<'_> = Insert::multi_into(table.clone(), vec![id, name])
            .values(vec![val!(1), val!("Musti")])
            .values(vec![val!(1), val!("Naukio")])
            .into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(1, changes);

        let select = Select::from_table(table).order_by("id".ascend());

        let res = connection.select(select).await?;
        assert_eq!(2, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        let row = res.get(1).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_compoud_unique_with_default() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();
        let index_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT, name NVARCHAR(4000) DEFAULT 'Musti')",
                table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!(
                "CREATE UNIQUE INDEX {} ON {} (id ASC, name ASC)",
                index_name, table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (1,'Musti')", table_name))
            .await?;

        let id = Column::from("id").table(&table_name);
        let name = Column::from("name").table(&table_name).default("Musti");

        let table = Table::from(&table_name).add_unique_index(vec![id.clone(), name.clone()]);

        let insert: Insert<'_> = Insert::single_into(table.clone()).value(id, 1).into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(0, changes);

        let select = Select::from_table(table).order_by("id".ascend());

        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_unique_with_autogen() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT NOT NULL IDENTITY(1,1) PRIMARY KEY, name VARCHAR(100))",
                table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (name) VALUES ('Musti')", table_name))
            .await?;

        let id = Column::from("id").table(&table_name).default(DefaultValue::Generated);
        let name = Column::from("name").table(&table_name);

        let table = Table::from(&table_name).add_unique_index(vec![id.clone(), name.clone()]);

        let insert: Insert<'_> = Insert::single_into(table.clone()).value(name, "Naukio").into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(1, changes);

        let select = Select::from_table(table).order_by("id".ascend());

        let res = connection.select(select).await?;
        assert_eq!(2, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        let row = res.get(1).unwrap();
        assert_eq!(Some(2), row["id"].as_i64());
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn single_insert_conflict_do_nothing_compoud_unique_with_autogen_default() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();
        let index_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (id INT IDENTITY(1,1) PRIMARY KEY, name NVARCHAR(4000) DEFAULT 'Musti')",
                table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!(
                "CREATE UNIQUE INDEX {} ON {} (id ASC, name ASC)",
                index_name, table_name,
            ))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (name) VALUES ('Musti')", table_name))
            .await?;

        let id = Column::from("id").table(&table_name).default(DefaultValue::Generated);
        let name = Column::from("name").table(&table_name).default("Musti");

        let table = Table::from(&table_name).add_unique_index(vec![id.clone(), name.clone()]);

        let insert: Insert<'_> = Insert::single_into(table.clone()).value(name, "Musti").into();

        let changes = connection
            .execute(insert.on_conflict(OnConflict::DoNothing).into())
            .await?;

        assert_eq!(1, changes);

        let select = Select::from_table(table).order_by("id".ascend());

        let res = connection.select(select).await?;
        assert_eq!(2, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        let row = res.get(1).unwrap();
        assert_eq!(Some(2), row["id"].as_i64());
        assert_eq!(Some("Musti"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn updates() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(4000))", table_name,))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (1,'Musti')", table_name))
            .await?;

        let update = Update::table(&table_name).set("name", "Naukio").so_that("id".equals(1));
        let changes = connection.execute(update.into()).await?;

        assert_eq!(1, changes);

        let select = Select::from_table(&table_name).order_by("id".ascend());
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.get(0).unwrap();
        assert_eq!(Some(1), row["id"].as_i64());
        assert_eq!(Some("Naukio"), row["name"].as_str());

        Ok(())
    }

    #[tokio::test]
    async fn deletes() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (id INT, name NVARCHAR(4000))", table_name,))
            .await?;

        connection
            .raw_cmd(&format!("INSERT INTO {} (id,name) VALUES (1,'Musti')", table_name))
            .await?;

        let delete = Delete::from_table(&table_name).so_that("id".equals(1));
        let changes = connection.execute(delete.into()).await?;

        assert_eq!(1, changes);

        let select = Select::from_table(&table_name).order_by("id".ascend());
        let res = connection.select(select).await?;
        assert_eq!(0, res.len());

        Ok(())
    }

    #[tokio::test]
    async fn integer_mapping() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (foo INT)", table_name))
            .await?;

        let insert = Insert::single_into(&table_name).value("foo", Value::integer(1));
        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::integer(1), row["foo"]);

        Ok(())
    }

    #[tokio::test]
    async fn real_mapping() -> crate::Result<()> {
        let decimal = Decimal::new(2122, 2);
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (foo DECIMAL(4,2))", table_name))
            .await?;

        let insert = Insert::single_into(&table_name).value("foo", Value::real(decimal));
        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::real(decimal), row["foo"]);

        Ok(())
    }

    #[tokio::test]
    async fn text_mapping() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (a NVARCHAR(10), b VARCHAR(10), c NTEXT, d TEXT)",
                table_name
            ))
            .await?;

        let insert = Insert::single_into(&table_name)
            .value("a", Value::text("äiti"))
            .value("b", Value::text("äiti"))
            .value("c", Value::text("äiti"))
            .value("d", Value::text("aeiti"));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::text("äiti"), row["a"]);
        assert_eq!(Value::text("äiti"), row["b"]);
        assert_eq!(Value::text("äiti"), row["c"]);
        assert_eq!(Value::text("aeiti"), row["d"]);

        Ok(())
    }

    #[tokio::test]
    async fn bytes_mapping() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();
        let data = vec![1, 2, 3];

        connection
            .raw_cmd(&format!(
                "CREATE TABLE {} (a binary(3), b varbinary(100), c image)",
                table_name
            ))
            .await?;

        let insert = Insert::single_into(&table_name)
            .value("a", Value::bytes(&data))
            .value("b", Value::bytes(&data))
            .value("c", Value::bytes(&data));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::bytes(&data), row["a"]);
        assert_eq!(Value::bytes(&data), row["b"]);
        assert_eq!(Value::bytes(&data), row["c"]);

        Ok(())
    }

    #[tokio::test]
    async fn boolean_mapping() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (a bit, b bit)", table_name))
            .await?;

        let insert = Insert::single_into(&table_name)
            .value("a", Value::boolean(true))
            .value("b", Value::boolean(false));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::boolean(true), row["a"]);
        assert_eq!(Value::boolean(false), row["b"]);

        Ok(())
    }

    #[tokio::test]
    async fn char_mapping() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (a char, b nchar)", table_name))
            .await?;

        let insert = Insert::single_into(&table_name)
            .value("a", Value::character('a'))
            .value("b", Value::character('ä'));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::text("a"), row["a"]);
        assert_eq!(Value::text("ä"), row["b"]);

        Ok(())
    }

    #[tokio::test]
    async fn json_mapping() -> crate::Result<()> {
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (a nvarchar(max))", table_name))
            .await?;

        let insert = Insert::single_into(&table_name).value("a", Value::json(json!({"foo":"bar"})));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::text("{\"foo\":\"bar\"}"), row["a"]);

        Ok(())
    }

    #[tokio::test]
    async fn uuid_mapping() -> crate::Result<()> {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (a uniqueidentifier)", table_name))
            .await?;

        let insert = Insert::single_into(&table_name).value("a", Value::uuid(uuid));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::uuid(uuid), row["a"]);

        Ok(())
    }

    #[tokio::test]
    async fn datetime_mapping() -> crate::Result<()> {
        let dt: DateTime<Utc> = DateTime::parse_from_rfc3339("2020-06-02T16:53:57.223231500Z")
            .unwrap()
            .into();

        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (a datetimeoffset, b datetime2)", table_name))
            .await?;

        let insert = Insert::single_into(&table_name)
            .value("a", Value::datetime(dt))
            .value("b", Value::datetime(dt));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::datetime(dt), row["a"]);
        assert_eq!(Value::datetime(dt), row["b"]);

        Ok(())
    }

    #[tokio::test]
    async fn date_mapping() -> crate::Result<()> {
        let date = NaiveDate::from_ymd(2020, 6, 2);

        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (a date)", table_name))
            .await?;

        let insert = Insert::single_into(&table_name).value("a", Value::date(date));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::date(date), row["a"]);

        Ok(())
    }

    #[tokio::test]
    async fn time_mapping() -> crate::Result<()> {
        let time = NaiveTime::from_hms(16, 20, 0);

        let connection = single::Quaint::new(&CONN_STR).await?;
        let table_name = random_table();

        connection
            .raw_cmd(&format!("CREATE TABLE {} (a time)", table_name))
            .await?;

        let insert = Insert::single_into(&table_name).value("a", Value::time(time));

        assert_eq!(1, connection.execute(insert.into()).await?);

        let select = Select::from_table(&table_name);
        let res = connection.select(select).await?;
        assert_eq!(1, res.len());

        let row = res.into_single()?;
        assert_eq!(Value::time(time), row["a"]);

        Ok(())
    }
}
