use crate::connector_loader::load_connector;
use crate::error::{render_panic, CoreError};
use introspection_connector::DatabaseMetadata;
use jsonrpc_core::*;
use jsonrpc_derive::rpc;
use tokio::runtime::Runtime;
use std::future::Future as StdFuture;

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
    runtime: Runtime,
}

impl Rpc for RpcImpl {
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>> {
        self.block_on(Self::list_databases_internal(url))
    }

    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata> {
        self.block_on(Self::get_database_metadata_internal(url))
    }

    fn introspect(&self, url: UrlInput) -> Result<String> {
        self.block_on(Self::introspect_internal(url))
    }
}

impl RpcImpl {
    pub(crate) fn new() -> Self {
        RpcImpl {
            runtime: Runtime::new().unwrap(),
        }
    }

    pub(crate) async fn introspect_internal(url: UrlInput) -> Result<String> {
        let connector = load_connector(&url.url).await?;
        let data_model = connector.introspect("").await.map_err(CoreError::from)?;
        Ok(datamodel::render_datamodel_to_string(&data_model).map_err(CoreError::from)?)
    }

    pub(crate) async fn list_databases_internal(url: UrlInput) -> Result<Vec<String>> {
        let connector = load_connector(&url.url).await?;
        Ok(connector.list_databases().await.map_err(CoreError::from)?)
    }

    pub(crate) async fn get_database_metadata_internal(url: UrlInput) -> Result<DatabaseMetadata> {
        let connector = load_connector(&url.url).await?;
        Ok(connector.get_metadata("").await.map_err(CoreError::from)?)
    }

    /// Will also catch panics.
    fn block_on<O>(&self, fut: impl StdFuture<Output = Result<O>>) -> Result<O> {
        use std::panic::{AssertUnwindSafe};
        use futures03::FutureExt;

        match self.runtime.block_on(AssertUnwindSafe(fut).catch_unwind()) {
            Ok(Ok(o)) => Ok(o),
            Ok(Err(err)) => Err(err),
            Err(err) => Err(render_panic(err)),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct UrlInput {
    pub(crate) url: String,
}
