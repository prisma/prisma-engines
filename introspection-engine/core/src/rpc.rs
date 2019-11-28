use crate::connector_loader::load_connector;
use crate::error::{CoreError};
use introspection_connector::DatabaseMetadata;
use jsonrpc_core::*;
use jsonrpc_derive::rpc;
use tokio::runtime::Runtime;
use std::future::Future as StdFuture;

#[rpc]
pub trait Rpc {
    #[rpc(name = "listDatabases")]
    fn list_databases(&self, connection_string: String) -> Result<Vec<String>>;

    #[rpc(name = "getDatabaseMetadata")]
    fn get_database_metadata(&self, connection_string: String) -> Result<DatabaseMetadata>;

    #[rpc(name = "introspect")]
    fn introspect(&self, connection_string: String) -> Result<String>;
}

pub(crate) struct RpcImpl {
    runtime: Runtime,
}

impl Rpc for RpcImpl {
    fn list_databases(&self, connection_string: String) -> Result<Vec<String>> {
        self.block_on(Self::list_databases_internal(connection_string))
    }

    fn get_database_metadata(&self, connection_string: String) -> Result<DatabaseMetadata> {
        self.block_on(Self::get_database_metadata_internal(connection_string))
    }

    fn introspect(&self, connection_string: String) -> Result<String> {
        self.block_on(Self::introspect_internal(connection_string))
    }
}

impl RpcImpl {
    pub(crate) fn new() -> Self {
        RpcImpl {
            runtime: Runtime::new().unwrap(),
        }
    }

    pub(crate) async fn introspect_internal(connection_string: String) -> Result<String> {
        let connector = load_connector(connection_string.as_str()).await?;
        let data_model = connector.introspect().await.map_err(CoreError::from)?;
        Ok(datamodel::render_datamodel_to_string(&data_model).map_err(CoreError::from)?)
    }

    pub(crate) async fn list_databases_internal(connection_string: String) -> Result<Vec<String>> {
        let connector = load_connector(connection_string.as_str()).await?;
        Ok(connector.list_databases().await.map_err(CoreError::from)?)
    }

    pub(crate) async fn get_database_metadata_internal(connection_string: String) -> Result<DatabaseMetadata> {
        let connector = load_connector(connection_string.as_str()).await?;
        Ok(connector.get_metadata().await.map_err(CoreError::from)?)
    }

    /// Will also catch panics.
    fn block_on<O>(&self, fut: impl StdFuture<Output = Result<O>>) -> Result<O> {
        match self.runtime.block_on(fut) {
            Ok(o) => Ok(o),
            Err(err) => Err(err),
        }
    }
}
