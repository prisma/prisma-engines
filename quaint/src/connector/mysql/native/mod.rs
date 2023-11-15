//! Definitions for the MySQL connector.
//! This module is not compatible with wasm32-* targets.
//! This module is only available with the `mysql-native` feature.
mod conversion;
mod error;

pub(crate) use crate::connector::mysql::MysqlUrl;
use crate::connector::{timeout, IsolationLevel};

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use lru_cache::LruCache;
use mysql_async::{
    self as my,
    prelude::{Query as _, Queryable as _},
};
use std::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::sync::Mutex;

/// The underlying MySQL driver. Only available with the `expose-drivers`
/// Cargo feature.
#[cfg(feature = "expose-drivers")]
pub use mysql_async;

impl MysqlUrl {
    pub(crate) fn cache(&self) -> LruCache<String, my::Statement> {
        LruCache::new(self.query_params.statement_cache_size)
    }

    pub(crate) fn to_opts_builder(&self) -> my::OptsBuilder {
        let mut config = my::OptsBuilder::default()
            .stmt_cache_size(Some(0))
            .user(Some(self.username()))
            .pass(self.password())
            .db_name(Some(self.dbname()));

        match self.socket() {
            Some(ref socket) => {
                config = config.socket(Some(socket));
            }
            None => {
                config = config.ip_or_hostname(self.host()).tcp_port(self.port());
            }
        }

        config = config.conn_ttl(Some(Duration::from_secs(5)));

        if self.query_params.use_ssl {
            config = config.ssl_opts(Some(self.query_params.ssl_opts.clone()));
        }

        if self.query_params.prefer_socket.is_some() {
            config = config.prefer_socket(self.query_params.prefer_socket);
        }

        config
    }
}

/// A connector interface for the MySQL database.
#[derive(Debug)]
pub struct Mysql {
    pub(crate) conn: Mutex<my::Conn>,
    pub(crate) url: MysqlUrl,
    socket_timeout: Option<Duration>,
    is_healthy: AtomicBool,
    statement_cache: Mutex<LruCache<String, my::Statement>>,
}

impl Mysql {
    /// Create a new MySQL connection using `OptsBuilder` from the `mysql` crate.
    pub async fn new(url: MysqlUrl) -> crate::Result<Self> {
        let conn = timeout::connect(url.connect_timeout(), my::Conn::new(url.to_opts_builder())).await?;

        Ok(Self {
            socket_timeout: url.query_params.socket_timeout,
            conn: Mutex::new(conn),
            statement_cache: Mutex::new(url.cache()),
            url,
            is_healthy: AtomicBool::new(true),
        })
    }

    /// The underlying mysql_async::Conn. Only available with the
    /// `expose-drivers` Cargo feature. This is a lower level API when you need
    /// to get into database specific features.
    #[cfg(feature = "expose-drivers")]
    pub fn conn(&self) -> &Mutex<mysql_async::Conn> {
        &self.conn
    }

    async fn perform_io<F, U, T>(&self, op: U) -> crate::Result<T>
    where
        F: Future<Output = crate::Result<T>>,
        U: FnOnce() -> F,
    {
        match timeout::socket(self.socket_timeout, op()).await {
            Err(e) if e.is_closed() => {
                self.is_healthy.store(false, Ordering::SeqCst);
                Err(e)
            }
            res => Ok(res?),
        }
    }

    async fn prepared<F, U, T>(&self, sql: &str, op: U) -> crate::Result<T>
    where
        F: Future<Output = crate::Result<T>>,
        U: Fn(my::Statement) -> F,
    {
        if self.url.statement_cache_size() == 0 {
            self.perform_io(|| async move {
                let stmt = {
                    let mut conn = self.conn.lock().await;
                    conn.prep(sql).await?
                };

                let res = op(stmt.clone()).await;

                {
                    let mut conn = self.conn.lock().await;
                    conn.close(stmt).await?;
                }

                res
            })
            .await
        } else {
            self.perform_io(|| async move {
                let stmt = self.fetch_cached(sql).await?;
                op(stmt).await
            })
            .await
        }
    }

    async fn fetch_cached(&self, sql: &str) -> crate::Result<my::Statement> {
        let mut cache = self.statement_cache.lock().await;
        let capacity = cache.capacity();
        let stored = cache.len();

        match cache.get_mut(sql) {
            Some(stmt) => {
                tracing::trace!(
                    message = "CACHE HIT!",
                    query = sql,
                    capacity = capacity,
                    stored = stored,
                );

                Ok(stmt.clone()) // arc'd
            }
            None => {
                tracing::trace!(
                    message = "CACHE MISS!",
                    query = sql,
                    capacity = capacity,
                    stored = stored,
                );

                let mut conn = self.conn.lock().await;
                if cache.capacity() == cache.len() {
                    if let Some((_, stmt)) = cache.remove_lru() {
                        conn.close(stmt).await?;
                    }
                }

                let stmt = conn.prep(sql).await?;
                cache.insert(sql.to_string(), stmt.clone());

                Ok(stmt)
            }
        }
    }
}

impl_default_TransactionCapable!(Mysql);

#[async_trait]
impl Queryable for Mysql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Mysql::build(q)?;
        self.query_raw(&sql, &params).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        metrics::query("mysql.query_raw", sql, params, move || async move {
            self.prepared(sql, |stmt| async move {
                let mut conn = self.conn.lock().await;
                let rows: Vec<my::Row> = conn.exec(&stmt, conversion::conv_params(params)?).await?;
                let columns = stmt.columns().iter().map(|s| s.name_str().into_owned()).collect();

                let last_id = conn.last_insert_id();
                let mut result_set = ResultSet::new(columns, Vec::new());

                for mut row in rows {
                    result_set.rows.push(row.take_result_row()?);
                }

                if let Some(id) = last_id {
                    result_set.set_last_insert_id(id);
                };

                Ok(result_set)
            })
            .await
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.query_raw(sql, params).await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Mysql::build(q)?;
        self.execute_raw(&sql, &params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("mysql.execute_raw", sql, params, move || async move {
            self.prepared(sql, |stmt| async move {
                let mut conn = self.conn.lock().await;
                conn.exec_drop(stmt, conversion::conv_params(params)?).await?;

                Ok(conn.affected_rows())
            })
            .await
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        metrics::query("mysql.raw_cmd", cmd, &[], move || async move {
            self.perform_io(|| async move {
                let mut conn = self.conn.lock().await;
                let mut result = cmd.run(&mut *conn).await?;

                loop {
                    result.map(drop).await?;

                    if result.is_empty() {
                        result.map(drop).await?;
                        break;
                    }
                }

                Ok(())
            })
            .await
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT @@GLOBAL.version version"#;
        let rows = timeout::socket(self.socket_timeout, self.query_raw(query, &[])).await?;

        let version_string = rows
            .get(0)
            .and_then(|row| row.get("version").and_then(|version| version.typed.to_string()));

        Ok(version_string)
    }

    fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        if matches!(isolation_level, IsolationLevel::Snapshot) {
            return Err(Error::builder(ErrorKind::invalid_isolation_level(&isolation_level)).build());
        }

        self.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation_level}"))
            .await?;

        Ok(())
    }

    fn requires_isolation_first(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::MysqlUrl;
    use crate::tests::test_api::mysql::CONN_STR;
    use crate::{error::*, single::Quaint};
    use url::Url;

    #[test]
    fn should_parse_socket_url() {
        let url = MysqlUrl::new(Url::parse("mysql://root@localhost/dbname?socket=(/tmp/mysql.sock)").unwrap()).unwrap();
        assert_eq!("dbname", url.dbname());
        assert_eq!(&Some(String::from("/tmp/mysql.sock")), url.socket());
    }

    #[test]
    fn should_parse_prefer_socket() {
        let url =
            MysqlUrl::new(Url::parse("mysql://root:root@localhost:3307/testdb?prefer_socket=false").unwrap()).unwrap();
        assert!(!url.prefer_socket().unwrap());
    }

    #[test]
    fn should_parse_sslaccept() {
        let url =
            MysqlUrl::new(Url::parse("mysql://root:root@localhost:3307/testdb?sslaccept=strict").unwrap()).unwrap();
        assert!(url.query_params.use_ssl);
        assert!(!url.query_params.ssl_opts.skip_domain_validation());
        assert!(!url.query_params.ssl_opts.accept_invalid_certs());
    }

    #[test]
    fn should_parse_ipv6_host() {
        let url = MysqlUrl::new(Url::parse("mysql://[2001:db8:1234::ffff]:5432/testdb").unwrap()).unwrap();
        assert_eq!("2001:db8:1234::ffff", url.host());
    }

    #[test]
    fn should_allow_changing_of_cache_size() {
        let url = MysqlUrl::new(Url::parse("mysql:///root:root@localhost:3307/foo?statement_cache_size=420").unwrap())
            .unwrap();
        assert_eq!(420, url.cache().capacity());
    }

    #[test]
    fn should_have_default_cache_size() {
        let url = MysqlUrl::new(Url::parse("mysql:///root:root@localhost:3307/foo").unwrap()).unwrap();
        assert_eq!(100, url.cache().capacity());
    }

    #[tokio::test]
    async fn should_map_nonexisting_database_error() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.set_username("root").unwrap();
        url.set_path("/this_does_not_exist");

        let url = url.as_str().to_string();
        let res = Quaint::new(&url).await;

        let err = res.unwrap_err();

        match err.kind() {
            ErrorKind::DatabaseDoesNotExist { db_name } => {
                assert_eq!(Some("1049"), err.original_code());
                assert_eq!(Some("Unknown database \'this_does_not_exist\'"), err.original_message());
                assert_eq!(&Name::available("this_does_not_exist"), db_name)
            }
            e => panic!("Expected `DatabaseDoesNotExist`, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn should_map_wrong_credentials_error() {
        let mut url = Url::parse(&CONN_STR).unwrap();
        url.set_username("WRONG").unwrap();

        let res = Quaint::new(url.as_str()).await;
        assert!(res.is_err());

        let err = res.unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::AuthenticationFailed { user } if user == &Name::available("WRONG")));
    }
}
