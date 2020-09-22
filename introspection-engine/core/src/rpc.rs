use crate::command_error::CommandError;
use crate::error::Error;
use datamodel::{Configuration, Datamodel};
use futures::{FutureExt, TryFutureExt};
use introspection_connector::{ConnectorResult, DatabaseMetadata, IntrospectionConnector, IntrospectionResultOutput};
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

    #[rpc(name = "getDatabaseVersion")]
    fn get_database_version(&self, input: IntrospectionInput) -> RpcFutureResult<String>;

    #[rpc(name = "introspect")]
    fn introspect(&self, input: IntrospectionInput) -> RpcFutureResult<IntrospectionResultOutput>;
}

pub struct RpcImpl;

impl Rpc for RpcImpl {
    fn list_databases(&self, input: IntrospectionInput) -> RpcFutureResult<Vec<String>> {
        Box::new(Self::list_databases_internal(input.schema).boxed().compat())
    }

    fn get_database_metadata(&self, input: IntrospectionInput) -> RpcFutureResult<DatabaseMetadata> {
        Box::new(Self::get_database_metadata_internal(input.schema).boxed().compat())
    }

    fn get_database_description(&self, input: IntrospectionInput) -> RpcFutureResult<String> {
        Box::new(Self::get_database_description_internal(input.schema).boxed().compat())
    }

    fn get_database_version(&self, input: IntrospectionInput) -> RpcFutureResult<String> {
        Box::new(Self::get_database_version_internal(input.schema).boxed().compat())
    }

    fn introspect(&self, input: IntrospectionInput) -> RpcFutureResult<IntrospectionResultOutput> {
        Box::new(Self::introspect_internal(input.schema, input.force).boxed().compat())
    }
}

impl RpcImpl {
    pub fn new() -> Self {
        RpcImpl
    }

    async fn load_connector(
        schema: &String,
    ) -> Result<(Configuration, String, Box<dyn IntrospectionConnector>), Error> {
        let config = datamodel::parse_configuration(&schema)?;

        let url = config
            .datasources
            .first()
            .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?
            .url()
            .to_owned()
            .value;

        Ok((
            config,
            url.clone(),
            Box::new(SqlIntrospectionConnector::new(&url).await?),
        ))
    }

    pub async fn catch<O>(fut: impl std::future::Future<Output = ConnectorResult<O>>) -> RpcResult<O> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(e) => Err(RpcError::from(Error::from(e))),
        }
    }

    pub async fn introspect_internal(schema: String, force: bool) -> RpcResult<IntrospectionResultOutput> {
        let (config, url, connector) = RpcImpl::load_connector(&schema).await?;

        let input_data_model = if !force {
            datamodel::parse_datamodel(&schema).map_err(|err| {
                Error::from(CommandError::ReceivedBadDatamodel(
                    err.to_pretty_string("schema.prisma", &schema),
                ))
            })?
        } else {
            Datamodel::new()
        };

        let result = match connector.introspect(&input_data_model).await {
            Ok(introspection_result) => {
                if introspection_result.data_model.is_empty() {
                    Err(Error::from(CommandError::IntrospectionResultEmpty(url.to_string())))
                } else {
                    match datamodel::render_datamodel_and_config_to_string(&introspection_result.data_model, &config) {
                        Err(e) => Err(Error::from(e)),
                        Ok(dm) => Ok(IntrospectionResultOutput {
                            datamodel: dm,
                            warnings: introspection_result.warnings,
                            version: introspection_result.version,
                        }),
                    }
                }
            }
            Err(e) => Err(Error::from(e)),
        };

        result.map_err(RpcError::from)
    }

    pub async fn list_databases_internal(schema: String) -> RpcResult<Vec<String>> {
        let (_, _, connector) = RpcImpl::load_connector(&schema).await?;
        RpcImpl::catch(connector.list_databases()).await
    }

    pub async fn get_database_description_internal(schema: String) -> RpcResult<String> {
        let (_, _, connector) = RpcImpl::load_connector(&schema).await?;
        RpcImpl::catch(connector.get_database_description()).await
    }

    pub async fn get_database_version_internal(schema: String) -> RpcResult<String> {
        let (_, _, connector) = RpcImpl::load_connector(&schema).await?;
        RpcImpl::catch(connector.get_database_version()).await
    }

    pub async fn get_database_metadata_internal(schema: String) -> RpcResult<DatabaseMetadata> {
        let (_, _, connector) = RpcImpl::load_connector(&schema).await?;
        RpcImpl::catch(connector.get_metadata()).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectionInput {
    pub(crate) schema: String,
    #[serde(default = "default_false")]
    pub(crate) force: bool,
}

fn default_false() -> bool {
    false
}
