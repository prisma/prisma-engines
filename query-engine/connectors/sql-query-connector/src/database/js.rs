use super::connection::SqlConnection;
use crate::FromSource;
use async_trait::async_trait;
use connector_interface::{
    self as connector,
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use quaint::{
    connector::IsolationLevel,
    prelude::{Queryable as QuaintQueryable, *},
};
use std::sync::Arc;

// TODO: https://github.com/prisma/team-orm/issues/245
// implement registry for client drivers, rather than a global variable,
// this would require the register_driver and registered_js_driver functions to
// receive an identifier for the specific driver
static QUERYABLE: once_cell::sync::OnceCell<Arc<dyn Queryable>> = once_cell::sync::OnceCell::new();

pub fn registered_js_connector() -> Option<&'static Arc<dyn Queryable>> {
    QUERYABLE.get()
}

pub fn register_js_connector(driver: Arc<dyn Queryable>) {
    if QUERYABLE.set(driver).is_err() {
        panic!("Cannot register driver twice");
    }
}

pub struct Js {
    connector: JsConnector,
    connection_info: ConnectionInfo,
    features: psl::PreviewFeatures,
    psl_connector: psl::builtin_connectors::JsConnector,
}

fn get_connection_info(url: &str) -> connector::Result<ConnectionInfo> {
    ConnectionInfo::from_url(url).map_err(|err| {
        ConnectorError::from_kind(ErrorKind::InvalidDatabaseUrl {
            details: err.to_string(),
            url: url.to_string(),
        })
    })
}

#[async_trait]
impl FromSource for Js {
    async fn from_source(
        source: &psl::Datasource,
        url: &str,
        features: psl::PreviewFeatures,
    ) -> connector_interface::Result<Js> {
        let psl_connector = source.active_connector.as_js_connector().unwrap_or_else(|| {
            panic!(
                "Connector for {} is not a JsConnector",
                source.active_connector.provider_name()
            )
        });

        let connector = registered_js_connector().unwrap().clone();
        let connection_info = get_connection_info(url)?;

        return Ok(Js {
            connector: JsConnector { queryable: connector },
            connection_info,
            features: features.to_owned(),
            psl_connector,
        });
    }
}

#[async_trait]
impl Connector for Js {
    async fn get_connection<'a>(&'a self) -> connector::Result<Box<dyn Connection + Send + Sync + 'static>> {
        super::catch(self.connection_info.clone(), async move {
            let sql_conn = SqlConnection::new(self.connector.clone(), &self.connection_info, self.features);
            Ok(Box::new(sql_conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> &'static str {
        self.psl_connector.name
    }

    fn should_retry_on_transient_error(&self) -> bool {
        false
    }
}

// TODO: miguelff: I haven´t found a better way to do this, yet... please continue reading.
//
// There is a bug in NAPI-rs by wich compiling a binary crate that links code using napi-rs
// bindings breaks. We could have used a JsQueryable from the `js-connectors` crate directly, as the
// `connection` field of a `Js` connector, but that will imply using napi-rs transitively, and break
// the tests (which are compiled as binary creates)
//
// To avoid the problem above I separated interface from implementation, making JsConnector
// independent on napi-rs. Initially, I tried having a field Arc<&dyn TransactionCabable> to hold
// JsQueryable at runtime. I did this, because TransactionCapable is the trait bounds required to
// create a value of  `SqlConnection` (see [SqlConnection::new])) to actually performt the queries.
// using JSQueryable. However, this didn't work because TransactionCapable is not object safe.
// (has Sized as a supertrait)
//
// The thing is that TransactionCapable is not object safe and cannot be used in a dynamic type
// declaration, so finally I couldn't come up with anything better then wrapping a QuaintQueryable
// in this object, and implementing TransactionCapable (and quaint::Queryable) explicitly for it.
#[derive(Clone)]
struct JsConnector {
    queryable: Arc<dyn QuaintQueryable>,
}

#[async_trait]
impl QuaintQueryable for JsConnector {
    async fn query(&self, q: Query<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        self.queryable.query(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        self.queryable.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        self.queryable.query_raw_typed(sql, params).await
    }

    async fn execute(&self, q: Query<'_>) -> quaint::Result<u64> {
        self.queryable.execute(q).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.queryable.execute_raw(sql, params).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.queryable.execute_raw_typed(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        self.queryable.raw_cmd(cmd).await
    }

    async fn version(&self) -> quaint::Result<Option<String>> {
        self.queryable.version().await
    }

    fn is_healthy(&self) -> bool {
        self.queryable.is_healthy()
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        self.queryable.set_tx_isolation_level(isolation_level).await
    }

    fn requires_isolation_first(&self) -> bool {
        self.queryable.requires_isolation_first()
    }
}

impl TransactionCapable for JsConnector {}
