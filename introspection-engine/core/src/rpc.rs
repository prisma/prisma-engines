use crate::connector_loader::load_connector;
use crate::error::CoreError;
use introspection_connector::DatabaseMetadata;
use jsonrpc_core::*;
use jsonrpc_derive::rpc;
use serde_derive::*;
use std::{future::Future as StdFuture, sync::Mutex};
use tokio::runtime::Runtime;
use tracing_futures::Instrument;

#[rpc]
pub trait Rpc {
    #[rpc(name = "listDatabases")]
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>>;

    #[rpc(name = "getDatabaseMetadata")]
    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata>;

    #[rpc(name = "introspect")]
    fn introspect(&self, url: UrlInput) -> Result<String>;
}

pub(crate) struct RpcImpl {
    runtime: Mutex<Runtime>,
}

impl Rpc for RpcImpl {
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>> {
        self.block_on(Self::list_databases_internal(&url.url))
    }

    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata> {
        self.block_on(Self::get_database_metadata_internal(&url.url))
    }

    fn introspect(&self, url: UrlInput) -> Result<String> {
        self.block_on(Self::introspect_internal(&url.url).instrument(tracing::info_span!("Introspect", ?url)))
    }
}

impl RpcImpl {
    pub(crate) fn new() -> Self {
        RpcImpl {
            runtime: Mutex::new(Runtime::new().unwrap()),
        }
    }

    pub(crate) async fn introspect_internal(connection_string: &str) -> Result<String> {
        let connector = load_connector(connection_string).await?;
        let data_model = connector.introspect().await.map_err(CoreError::from)?;
        Ok(datamodel::render_datamodel_to_string(&data_model).map_err(CoreError::from)?)
    }

    pub(crate) async fn list_databases_internal(connection_string: &str) -> Result<Vec<String>> {
        let connector = load_connector(connection_string).await?;
        Ok(connector.list_databases().await.map_err(CoreError::from)?)
    }

    pub(crate) async fn get_database_metadata_internal(connection_string: &str) -> Result<DatabaseMetadata> {
        let connector = load_connector(connection_string).await?;
        Ok(connector.get_metadata().await.map_err(CoreError::from)?)
    }

    /// Will also catch panics.
    fn block_on<O>(&self, fut: impl StdFuture<Output = Result<O>>) -> Result<O> {
        let mut rt = self.runtime.lock().unwrap();
        match rt.block_on(fut) {
            Ok(o) => Ok(o),
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UrlInput {
    pub(crate) url: String,
}
