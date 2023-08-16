use super::connection::SqlConnection;
use crate::FromSource;
use async_trait::async_trait;
use connector_interface::{
    self as connector,
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use once_cell::sync::Lazy;
use quaint::{
    connector::IsolationLevel,
    prelude::{Queryable as QuaintQueryable, *},
};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
};

/// Registry is the type for the global registry of Js connectors.
type Registry = HashMap<String, JsConnector>;

/// REGISTRY is the global registry of JsConnectors
static REGISTRY: Lazy<Mutex<Registry>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn registered_js_connector(provider: &str) -> connector::Result<JsConnector> {
    let lock = REGISTRY.lock().unwrap();
    lock.get(provider)
        .ok_or(ConnectorError::from_kind(ErrorKind::UnsupportedConnector(format!(
            "A Javascript connector proxy for {} was not registered",
            provider
        ))))
        .map(|conn_ref| conn_ref.to_owned())
}

pub fn register_js_connector(provider: &str, connector: Arc<dyn QuaintQueryable>) -> Result<(), String> {
    let mut lock = REGISTRY.lock().unwrap();
    let entry = lock.entry(provider.to_string());
    match entry {
        Entry::Occupied(_) => Err(format!(
            "A Javascript connector proxy for {} was already registered, and cannot be overridden.",
            provider
        )),
        Entry::Vacant(v) => {
            v.insert(JsConnector { connector });
            Ok(())
        }
    }
}

pub struct Js {
    connector: JsConnector,
    connection_info: ConnectionInfo,
    features: psl::PreviewFeatures,
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
        let connector = registered_js_connector(source.active_provider)?;
        let connection_info = get_connection_info(url)?;

        Ok(Js {
            connector,
            connection_info,
            features,
        })
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
        "js"
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
    connector: Arc<dyn QuaintQueryable>,
}

#[async_trait]
impl QuaintQueryable for JsConnector {
    async fn query(&self, q: Query<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        self.connector.query(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        self.connector.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        self.connector.query_raw_typed(sql, params).await
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

impl TransactionCapable for JsConnector {}
