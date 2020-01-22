use crate::connector_loader::load_connector;
use crate::error::CoreError;
use futures::{FutureExt, TryFutureExt};
use introspection_connector::DatabaseMetadata;
use jsonrpc_derive::rpc;
use serde_derive::*;

type RpcError = jsonrpc_core::Error;
type RpcResult<T> = Result<T, RpcError>;
type RpcFutureResult<T> = Box<dyn futures01::Future<Item = T, Error = RpcError> + Send + 'static>;

#[rpc]
pub trait Rpc {
    #[rpc(name = "listDatabases")]
    fn list_databases(&self, url: UrlInput) -> RpcFutureResult<Vec<String>>;

    #[rpc(name = "getDatabaseMetadata")]
    fn get_database_metadata(&self, url: UrlInput) -> RpcFutureResult<DatabaseMetadata>;

    #[rpc(name = "introspect")]
    fn introspect(&self, url: UrlInput) -> RpcFutureResult<String>;
}

pub(crate) struct RpcImpl;

impl Rpc for RpcImpl {
    fn list_databases(&self, url: UrlInput) -> RpcFutureResult<Vec<String>> {
        Box::new(Self::list_databases_internal(url.url).boxed().compat())
    }

    fn get_database_metadata(&self, url: UrlInput) -> RpcFutureResult<DatabaseMetadata> {
        Box::new(Self::get_database_metadata_internal(url.url).boxed().compat())
    }

    fn introspect(&self, url: UrlInput) -> RpcFutureResult<String> {
        Box::new(Self::introspect_internal(url.url).boxed().compat())
    }
}

impl RpcImpl {
    pub(crate) fn new() -> Self {
        RpcImpl
    }

    pub(crate) async fn introspect_internal(connection_string: String) -> RpcResult<String> {
        let connector = load_connector(&connection_string).await?;
        let data_model = connector.introspect().await.map_err(CoreError::from)?;
        Ok(datamodel::render_datamodel_to_string(&data_model).map_err(CoreError::from)?)
    }

    pub(crate) async fn list_databases_internal(connection_string: String) -> RpcResult<Vec<String>> {
        let connector = load_connector(&connection_string).await?;
        Ok(connector.list_databases().await.map_err(CoreError::from)?)
    }

    pub(crate) async fn get_database_metadata_internal(connection_string: String) -> RpcResult<DatabaseMetadata> {
        let connector = load_connector(&connection_string).await?;
        Ok(connector.get_metadata().await.map_err(CoreError::from)?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UrlInput {
    pub(crate) url: String,
}
