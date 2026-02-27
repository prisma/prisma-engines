//! Definitions for the MSSQL connector.
//! This module is not compatible with wasm32-* targets.
//! This module is only available with the `mssql-native` feature.
mod column_type;
mod conversion;
mod error;

pub(crate) use crate::connector::mssql::MssqlUrl;
use crate::connector::{DescribedQuery, IsolationLevel, Transaction, TransactionOptions, timeout};

use crate::{
    ast::{Query, Value},
    connector::{ColumnType as QuaintColumnType, DefaultTransaction, ResultSet, queryable::*, trace},
    visitor::{self, Visitor},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    convert::TryFrom,
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tiberius::*;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/// The underlying SQL Server driver. Only available with the `expose-drivers` Cargo feature.
#[cfg(feature = "expose-drivers")]
pub use tiberius;

static SQL_SERVER_DEFAULT_ISOLATION: IsolationLevel = IsolationLevel::ReadCommitted;
const DB_SYSTEM_NAME: &str = "mssql";

#[async_trait]
impl TransactionCapable for Mssql {
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> crate::Result<Box<dyn Transaction + 'a>> {
        // Isolation levels in SQL Server are set on the connection and live until they're changed.
        // Always explicitly setting the isolation level each time a tx is started (either to the given value
        // or by using the default/connection string value) prevents transactions started on connections from
        // the pool to have unexpected isolation levels set.
        let isolation = isolation
            .or(self.url.query_params.transaction_isolation_level)
            .or(Some(SQL_SERVER_DEFAULT_ISOLATION));

        let opts = TransactionOptions::new(isolation, self.requires_isolation_first());

        Ok(Box::new(
            DefaultTransaction::new(self, self.begin_statement(), opts).await?,
        ))
    }
}

/// A connector interface for the SQL Server database.
#[derive(Debug)]
pub struct Mssql {
    client: Mutex<Client<Compat<TcpStream>>>,
    url: MssqlUrl,
    socket_timeout: Option<Duration>,
    is_healthy: AtomicBool,
}

impl Mssql {
    /// Creates a new connection to SQL Server.
    pub async fn new(url: MssqlUrl) -> crate::Result<Self> {
        let mut config = Config::from_jdbc_string(&url.connection_string)?;
        config.database(url.dbname());
        let tcp = TcpStream::connect_named(&config).await?;
        let socket_timeout = url.socket_timeout();

        let connecting = async {
            match Client::connect(config, tcp.compat_write()).await {
                Ok(client) => Ok(client),
                Err(tiberius::error::Error::Routing { host, port }) => {
                    let mut config = Config::from_jdbc_string(&url.connection_string)?;
                    config.database(url.dbname());
                    config.host(host);
                    config.port(port);

                    let tcp = TcpStream::connect_named(&config).await?;
                    Client::connect(config, tcp.compat_write()).await
                }
                Err(e) => Err(e),
            }
        };

        let client = timeout::connect(url.connect_timeout(), connecting).await?;

        let this = Self {
            client: Mutex::new(client),
            url,
            socket_timeout,
            is_healthy: AtomicBool::new(true),
        };

        if let Some(isolation) = this.url.transaction_isolation_level() {
            this.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation}"))
                .await?;
        };

        Ok(this)
    }

    /// The underlying Tiberius client. Only available with the `expose-drivers` Cargo feature.
    /// This is a lower level API when you need to get into database specific features.
    #[cfg(feature = "expose-drivers")]
    pub fn client(&self) -> &Mutex<Client<Compat<TcpStream>>> {
        &self.client
    }

    async fn perform_io<F, T>(&self, fut: F) -> crate::Result<T>
    where
        F: Future<Output = std::result::Result<T, tiberius::error::Error>>,
    {
        match timeout::socket(self.socket_timeout, fut).await {
            Err(e) if e.is_closed() => {
                self.is_healthy.store(false, Ordering::SeqCst);
                Err(e)
            }
            res => res,
        }
    }
}

#[async_trait]
impl Queryable for Mssql {
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = visitor::Mssql::build(q)?;
        self.query_raw(&sql, &params[..]).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            let mut client = self.client.lock().await;

            let mut query = tiberius::Query::new(sql);

            for param in params {
                query.bind(param);
            }

            let mut results = self.perform_io(query.query(&mut client)).await?.into_results().await?;

            match results.pop() {
                Some(rows) => {
                    let mut columns_set = false;
                    let mut columns = Vec::new();

                    let mut types_set = false;
                    let mut types = Vec::new();

                    let mut result_rows = Vec::with_capacity(rows.len());

                    for row in rows.into_iter() {
                        if !columns_set {
                            columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                            columns_set = true;
                        }

                        if !types_set {
                            types = row.columns().iter().map(QuaintColumnType::from).collect();
                            types_set = true;
                        }

                        let mut values: Vec<Value<'_>> = Vec::with_capacity(row.len());

                        for val in row.into_iter() {
                            values.push(Value::try_from(val)?);
                        }

                        result_rows.push(values);
                    }

                    Ok(ResultSet::new(columns, types, result_rows))
                }
                None => Ok(ResultSet::new(Vec::new(), Vec::new(), Vec::new())),
            }
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        self.query_raw(sql, params).await
    }

    async fn describe_query(&self, _sql: &str) -> crate::Result<DescribedQuery> {
        unimplemented!("SQL Server does not support describe_query yet.")
    }

    async fn execute(&self, q: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = visitor::Mssql::build(q)?;
        self.execute_raw(&sql, &params[..]).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            let mut query = tiberius::Query::new(sql);

            for param in params {
                query.bind(param);
            }

            let mut client = self.client.lock().await;
            let changes = self.perform_io(query.execute(&mut client)).await?.total();

            Ok(changes)
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        self.execute_raw(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        trace::query(DB_SYSTEM_NAME, cmd, &[], move || async move {
            let mut client = self.client.lock().await;
            self.perform_io(client.simple_query(cmd)).await?.into_results().await?;
            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let query = r#"SELECT @@VERSION AS version"#;
        let rows = self.query_raw(query, &[]).await?;

        let version_string = rows
            .first()
            .and_then(|row| row.get("version").and_then(|version| version.to_string()));

        Ok(version_string)
    }

    fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        self.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation_level}"))
            .await?;

        Ok(())
    }

    fn begin_statement(&self) -> &'static str {
        "BEGIN TRAN"
    }

    fn requires_isolation_first(&self) -> bool {
        true
    }
}
