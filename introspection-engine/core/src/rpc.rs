use crate::command_error::CommandError;
use crate::error::Error;
use crate::error_rendering::render_jsonrpc_error;
use futures::{FutureExt, TryFutureExt};
use introspection_connector::{DatabaseMetadata, IntrospectionConnector};
use jsonrpc_derive::rpc;
use serde_derive::*;
use sql_introspection_connector::SqlIntrospectionConnector;

type RpcError = jsonrpc_core::Error;
type RpcResult<T> = Result<T, RpcError>;
type RpcFutureResult<T> = Box<dyn futures01::Future<Item = T, Error = RpcError> + Send + 'static>;

#[rpc]
pub trait Rpc {
    #[rpc(name = "listDatabases")]
    fn list_databases(&self, input: IntrospectionInput) -> RpcFutureResult<Vec<String>>;

    #[rpc(name = "getDatabaseMetadata")]
    fn get_database_metadata(&self, input: IntrospectionInput) -> RpcFutureResult<DatabaseMetadata>;

    #[rpc(name = "getDatabaseDescription")]
    fn get_database_description(&self, input: IntrospectionInput) -> RpcFutureResult<String>;

    #[rpc(name = "introspect")]
    fn introspect(&self, input: IntrospectionInput) -> RpcFutureResult<String>;
}

pub(crate) struct RpcImpl;

impl Rpc for RpcImpl {
    fn list_databases(&self, input: IntrospectionInput) -> RpcFutureResult<Vec<String>> {
        Box::new(Self::list_databases_internal(input.schema).boxed().compat())
    }

    fn get_database_metadata(&self, input: IntrospectionInput) -> RpcFutureResult<DatabaseMetadata> {
        Box::new(Self::get_database_metadata_internal(input.schema).boxed().compat())
    }

    fn get_database_description(&self, input: IntrospectionInput) -> RpcFutureResult<String> {
        Box::new(Self::get_database_description(input.schema).boxed().compat())
    }

    fn introspect(&self, input: IntrospectionInput) -> RpcFutureResult<String> {
        Box::new(Self::introspect_internal(input.schema).boxed().compat())
    }
}

impl RpcImpl {
    pub(crate) fn new() -> Self {
        RpcImpl
    }

    async fn load_connector(schema: &String) -> Result<Box<dyn IntrospectionConnector>, Error> {
        let config = datamodel::parse_configuration(&schema, false)?;
        let url = config
            .datasources
            .first()
            .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?
            .url()
            .to_owned()
            .value;
        Ok(Box::new(SqlIntrospectionConnector::new(&url).await?))
    }

    pub(crate) async fn introspect_internal(schema: String) -> RpcResult<String> {
        let config = datamodel::parse_configuration(&schema, false).map_err(Error::from)?;
        let url = config
            .datasources
            .first()
            .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))
            .map_err(Error::from)?
            .url()
            .to_owned()
            .value;
        let connector = RpcImpl::load_connector(&schema).await?;
        let data_model = connector.introspect().await;

        match data_model {
            Ok(dm) if dm.models.is_empty() && dm.enums.is_empty() => Err(render_jsonrpc_error(Error::from(
                CommandError::IntrospectionResultEmpty(url.to_string()),
            ))),
            Ok(dm) => Ok(datamodel::render_datamodel_and_config_to_string(&dm, &config).map_err(Error::from)?),
            Err(e) => Err(render_jsonrpc_error(Error::from(e))),
        }
    }

    pub(crate) async fn list_databases_internal(schema: String) -> RpcResult<Vec<String>> {
        let connector = RpcImpl::load_connector(&schema).await?;
        Ok(connector.list_databases().await.map_err(Error::from)?)
    }

    pub(crate) async fn get_database_description(schema: String) -> RpcResult<String> {
        let connector = RpcImpl::load_connector(&schema).await?;
        Ok(connector.get_database_description().await.map_err(Error::from)?)
    }

    pub(crate) async fn get_database_metadata_internal(schema: String) -> RpcResult<DatabaseMetadata> {
        let connector = RpcImpl::load_connector(&schema).await?;
        Ok(connector.get_metadata().await.map_err(Error::from)?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectionInput {
    pub(crate) schema: String,
}
