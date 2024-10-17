use super::connection::SqlConnection;
use async_trait::async_trait;
use connector_interface::{
    self as connector,
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use quaint::{
    connector::{ExternalConnector, IsolationLevel, Transaction},
    prelude::{Queryable as QuaintQueryable, *},
};
use std::sync::Arc;

pub struct Js {
    connector: DriverAdapter,
    connection_info: ConnectionInfo,
    features: psl::PreviewFeatures,
}

impl Js {
    pub async fn new(
        connector: Arc<dyn ExternalConnector>,
        features: psl::PreviewFeatures,
    ) -> connector_interface::Result<Self> {
        let external_conn_info = connector.get_connection_info().await.map_err(|e| match e.kind() {
            &quaint::error::ErrorKind::ExternalError(id) => ConnectorError::from_kind(ErrorKind::ExternalError(id)),
            _ => ConnectorError::from_kind(ErrorKind::InvalidDriverAdapter(
                "Error while calling getConnectionInfo()".into(),
            )),
        })?;

        Ok(Js {
            connector: DriverAdapter { connector },
            features,
            connection_info: ConnectionInfo::External(external_conn_info),
        })
    }
}

#[async_trait]
impl Connector for Js {
    async fn get_connection<'a>(&'a self) -> connector::Result<Box<dyn Connection + Send + Sync + 'static>> {
        super::catch(&self.connection_info, async move {
            let sql_conn = SqlConnection::new(self.connector.clone(), self.connection_info.clone(), self.features);
            Ok(Box::new(sql_conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> &'static str {
        match self.connection_info.sql_family() {
            #[cfg(feature = "postgresql")]
            SqlFamily::Postgres => "postgresql",
            #[cfg(feature = "mysql")]
            SqlFamily::Mysql => "mysql",
            #[cfg(feature = "sqlite")]
            SqlFamily::Sqlite => "sqlite",
            #[cfg(feature = "mssql")]
            SqlFamily::Mssql => "mssql",
        }
    }

    fn should_retry_on_transient_error(&self) -> bool {
        false
    }
}

/// There is a bug in NAPI-rs by wich compiling a binary crate that links code using napi-rs
/// bindings breaks. We could have used a JsQueryable from the `driver-adapters` crate directly, as the
/// `connection` field of a driver adapter, but that will imply using napi-rs transitively, and break
/// the tests (which are compiled as binary creates)
///
/// To avoid the problem above I separated interface from implementation, making DriverAdapter
/// independent on napi-rs. Initially, I tried having a field Arc<&dyn TransactionCabable> to hold
/// JsQueryable at runtime. I did this, because TransactionCapable is the trait bounds required to
/// create a value of  `SqlConnection` (see [SqlConnection::new])) to actually performt the queries.
/// using JSQueryable. However, this didn't work because TransactionCapable is not object safe.
/// (has Sized as a supertrait)
///
/// The thing is that TransactionCapable is not object safe and cannot be used in a dynamic type
/// declaration, so finally I couldn't come up with anything better then wrapping a QuaintQueryable
/// in this object, and implementing TransactionCapable (and quaint::Queryable) explicitly for it.
#[derive(Clone)]
pub struct DriverAdapter {
    connector: Arc<dyn ExternalConnector>,
}

#[async_trait]
impl QuaintQueryable for DriverAdapter {
    async fn query(&self, q: Query<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        self.connector.query(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        self.connector.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        self.connector.query_raw_typed(sql, params).await
    }

    async fn describe_query(&self, sql: &str) -> quaint::Result<quaint::connector::DescribedQuery> {
        self.connector.describe_query(sql).await
    }

    async fn execute(&self, q: Query<'_>) -> quaint::Result<u64> {
        self.connector.execute(q).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.connector.execute_raw(sql, params).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.connector.execute_raw_typed(sql, params).await
    }

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        self.connector.raw_cmd(cmd).await
    }

    async fn version(&self) -> quaint::Result<Option<String>> {
        self.connector.version().await
    }

    fn is_healthy(&self) -> bool {
        self.connector.is_healthy()
    }

    /// Sets the transaction isolation level to given value.
    /// Implementers have to make sure that the passed isolation level is valid for the underlying database.
    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        self.connector.set_tx_isolation_level(isolation_level).await
    }

    /// Signals if the isolation level SET needs to happen before or after the tx BEGIN.
    fn requires_isolation_first(&self) -> bool {
        self.connector.requires_isolation_first()
    }
}

#[async_trait]
impl TransactionCapable for DriverAdapter {
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> quaint::Result<Box<dyn Transaction + 'a>> {
        self.connector.start_transaction(isolation).await
    }
}
