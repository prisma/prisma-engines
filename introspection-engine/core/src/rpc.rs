use crate::connector_loader::load_connector;
use crate::CoreResult;
use datamodel::Datamodel;
use introspection_connector::DatabaseMetadata;
use jsonrpc_core::*;
use jsonrpc_derive::rpc;
use tokio::runtime::Runtime;

#[rpc]
pub trait Rpc {
    #[rpc(name = "listDatabases")]
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>>;

    #[rpc(name = "getDatabaseMetadata")]
    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata>;

    #[rpc(name = "introspect")]
    fn introspect(&self, url: UrlInput) -> Result<String>;
}

pub struct RpcImpl {
    runtime: Runtime,
}

impl Rpc for RpcImpl {
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>> {
        Ok(self.runtime.block_on(Self::list_databases_internal(url))?)
    }

    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata> {
        Ok(self.runtime.block_on(Self::get_database_metadata_internal(url))?)
    }

    fn introspect(&self, url: UrlInput) -> Result<String> {
        let data_model = self.runtime.block_on(Self::introspect_internal(url))?;
        Ok(datamodel::render_datamodel_to_string(&data_model).expect("Datamodel rendering failed"))
    }
}

impl RpcImpl {
    pub(crate) fn new() -> Self {
        RpcImpl {
            runtime: Runtime::new().unwrap(),
        }
    }

    async fn introspect_internal(url: UrlInput) -> CoreResult<Datamodel> {
        let connector = load_connector(&url.url)?;
        // FIXME: parse URL correctly via a to be built lib and pass database param;
        let data_model = connector.introspect("").await?;
        Ok(data_model)
    }

    async fn list_databases_internal(url: UrlInput) -> CoreResult<Vec<String>> {
        let connector = load_connector(&url.url)?;
        Ok(connector.list_databases().await?)
    }

    async fn get_database_metadata_internal(url: UrlInput) -> CoreResult<DatabaseMetadata> {
        let connector = load_connector(&url.url)?;
        Ok(connector.get_metadata("").await?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct UrlInput {
    url: String,
}
