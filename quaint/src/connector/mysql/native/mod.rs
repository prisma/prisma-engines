//! Definitions for the MySQL connector.
//! This module is not compatible with wasm32-* targets.
//! This module is only available with the `mysql-native` feature.
mod column_type;
mod conversion;
mod error;

pub(crate) use crate::connector::mysql::MysqlUrl;
use crate::connector::{timeout, ColumnType, DescribedColumn, DescribedParameter, DescribedQuery, IsolationLevel};

use crate::{
    ast::{Query, Value},
    connector::{metrics, queryable::*, ResultSet},
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use lru_cache::LruCache;
use mysql_async::consts::ColumnFlags;
use mysql_async::{
    self as my,
    prelude::{Query as _, Queryable as _},
};
use std::borrow::Cow;
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

const DB_SYSTEM_NAME: &str = "mysql";

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
        metrics::query("mysql.query_raw", DB_SYSTEM_NAME, sql, params, move || async move {
            self.prepared(sql, |stmt| async move {
                let mut conn = self.conn.lock().await;
                let rows: Vec<my::Row> = conn.exec(&stmt, conversion::conv_params(params)?).await?;

                let last_id = conn.last_insert_id();

                let mut result_rows = Vec::with_capacity(rows.len());
                let mut columns: Vec<String> = Vec::new();
                let mut column_types: Vec<ColumnType> = Vec::new();

                let mut columns_set = false;

                for mut row in rows {
                    let row = row.take_result_row()?;

                    if !columns_set {
                        for (idx, _) in row.iter().enumerate() {
                            let maybe_column = stmt.columns().get(idx);
                            // `mysql_async` does not return columns in `ResultSet` when a call to a stored procedure is done
                            // See https://github.com/prisma/prisma/issues/6173
                            let column = maybe_column
                                .map(|col| col.name_str().into_owned())
                                .unwrap_or_else(|| format!("f{idx}"));
                            let column_type = maybe_column.map(ColumnType::from).unwrap_or(ColumnType::Unknown);

                            columns.push(column);
                            column_types.push(column_type);
                        }

                        columns_set = true;
                    }

                    result_rows.push(row);
                }

                let mut result_set = ResultSet::new(columns, column_types, result_rows);

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

    async fn describe_query(&self, sql: &str) -> crate::Result<DescribedQuery> {
        self.prepared(sql, |stmt| async move {
            let columns = stmt
                .columns()
                .iter()
                .map(|col| {
                    DescribedColumn::new_named(col.name_str(), col)
                        .is_nullable(!col.flags().contains(ColumnFlags::NOT_NULL_FLAG))
                })
                .collect();
            let parameters = stmt
                .params()
                .iter()
                .enumerate()
                .map(|(idx, col)| DescribedParameter::new_unnamed(idx, col))
                .collect();

            Ok(DescribedQuery {
                columns,
                parameters,
                enum_names: None,
            })
        })
        .await
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Mysql::build(q)?;
        self.execute_raw(&sql, &params).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        metrics::query("mysql.execute_raw", DB_SYSTEM_NAME, sql, params, move || async move {
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
        metrics::query("mysql.raw_cmd", DB_SYSTEM_NAME, cmd, &[], move || async move {
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
        let guard = self.conn.lock().await;
        let (major, minor, patch) = guard.server_version();
        let flavour = if guard.is_mariadb() { "MariaDB" } else { "MySQL" };
        drop(guard);

        Ok(Some(format!("{major}.{minor}.{patch}-{flavour}")))
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

    /// Statement to begin a transaction
    fn begin_statement(&self, depth: u32) -> Cow<'static, str> {
        if depth > 1 {
            Cow::Owned(format!("SAVEPOINT savepoint{depth}"))
        } else {
            Cow::Borrowed("BEGIN")
        }
    }

    /// Statement to commit a transaction
    fn commit_statement(&self, depth: u32) -> Cow<'static, str> {
        if depth > 1 {
            Cow::Owned(format!("RELEASE SAVEPOINT savepoint{depth}"))
        } else {
            Cow::Borrowed("COMMIT")
        }
    }

    /// Statement to rollback a transaction
    fn rollback_statement(&self, depth: u32) -> Cow<'static, str> {
        if depth > 1 {
            Cow::Owned(format!("ROLLBACK TO savepoint{depth}"))
        } else {
            Cow::Borrowed("ROLLBACK")
        }
    }
}
