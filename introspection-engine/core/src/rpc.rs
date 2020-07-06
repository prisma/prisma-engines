use crate::command_error::CommandError;
use crate::error::Error;
use crate::error_rendering::render_jsonrpc_error;
use datamodel::{Configuration, Datamodel};
use futures::{FutureExt, TryFutureExt};
use introspection_connector::{DatabaseMetadata, IntrospectionConnector, IntrospectionResultOutput, Warning};
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
        Box::new(Self::get_database_description(input.schema).boxed().compat())
    }

    fn introspect(&self, input: IntrospectionInput) -> RpcFutureResult<IntrospectionResultOutput> {
        Box::new(
            Self::introspect_internal(input.schema, input.reintrospect)
                .boxed()
                .compat(),
        )
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

    pub async fn introspect_internal(schema: String, reintrospect: bool) -> RpcResult<IntrospectionResultOutput> {
        let (config, url, connector) = RpcImpl::load_connector(&schema)
            .await
            .map_err(|err| render_jsonrpc_error(err, &schema))?;

        let mut could_not_parse_input_data_model = false;
        let input_data_model = datamodel::parse_datamodel(&schema).unwrap_or_else(|_| {
            could_not_parse_input_data_model = true;
            Datamodel::new()
        });

        let result = match connector.introspect(&input_data_model, reintrospect).await {
            Ok(mut introspection_result) => {
                if introspection_result.datamodel.models.is_empty() && introspection_result.datamodel.enums.is_empty() {
                    Err(Error::from(CommandError::IntrospectionResultEmpty(url.to_string())))
                } else {
                    if could_not_parse_input_data_model && reintrospect {
                        introspection_result.warnings.push(Warning::new_datamodel_parsing())
                    };

                    match datamodel::render_datamodel_and_config_to_string(&introspection_result.datamodel, &config) {
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

        result.map_err(|e| render_jsonrpc_error(e, &schema))
    }

    pub async fn list_databases_internal(schema: String) -> RpcResult<Vec<String>> {
        let (_, _, connector) = RpcImpl::load_connector(&schema)
            .await
            .map_err(|e| render_jsonrpc_error(e, &schema))?;
        Ok(connector
            .list_databases()
            .await
            .map_err(|e| render_jsonrpc_error(Error::from(e), &schema))?)
    }

    pub async fn get_database_description(schema: String) -> RpcResult<String> {
        let (_, _, connector) = RpcImpl::load_connector(&schema)
            .await
            .map_err(|e| render_jsonrpc_error(e, &schema))?;
        Ok(connector
            .get_database_description()
            .await
            .map_err(|e| render_jsonrpc_error(Error::from(e), &schema))?)
    }

    pub async fn get_database_metadata_internal(schema: String) -> RpcResult<DatabaseMetadata> {
        let (_, _, connector) = RpcImpl::load_connector(&schema)
            .await
            .map_err(|err| render_jsonrpc_error(err, &schema))?;
        Ok(connector
            .get_metadata()
            .await
            .map_err(|e| render_jsonrpc_error(Error::from(e), &schema))?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectionInput {
    pub(crate) schema: String,
    #[serde(default = "default_false")]
    pub(crate) reintrospect: bool,
}

fn default_false() -> bool {
    false
}
