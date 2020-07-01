use crate::command_error::CommandError;
use crate::error::Error;
use crate::error_rendering::render_jsonrpc_error;
use datamodel::Datamodel;
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

    async fn load_connector(schema: &String) -> Result<Box<dyn IntrospectionConnector>, Error> {
        let config = datamodel::parse_configuration(&schema)?;
        let url = config
            .datasources
            .first()
            .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?
            .url()
            .to_owned()
            .value;

        Ok(Box::new(SqlIntrospectionConnector::new(&url).await?))
    }

    pub async fn introspect_internal(schema: String, reintrospect: bool) -> RpcResult<IntrospectionResultOutput> {
        let config = datamodel::parse_configuration(&schema).map_err(Error::from)?;
        let url = config
            .datasources
            .first()
            .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))
            .map_err(Error::from)?
            .url()
            .to_owned()
            .value;

        let connector = RpcImpl::load_connector(&schema).await?;

        let mut could_not_parse_input_data_model = false;
        let input_data_model = match datamodel::parse_datamodel(&schema) {
            Ok(existing_data_model) => existing_data_model,
            Err(_) => {
                could_not_parse_input_data_model = true;
                Datamodel::new()
            }
        };

        match connector.introspect(&input_data_model, reintrospect).await {
            Ok(introspection_result)
                if introspection_result.datamodel.models.is_empty()
                    && introspection_result.datamodel.enums.is_empty() =>
            {
                Err(render_jsonrpc_error(Error::from(
                    CommandError::IntrospectionResultEmpty(url.to_string()),
                )))
            }
            Ok(introspection_result) => {
                let warnings = match could_not_parse_input_data_model {
                    true if reintrospect => {
                        let mut warnings = introspection_result.warnings;
                        warnings.push(Warning {
                        code: 0,
                        message:
                        "The input datamodel could not be parsed. This means it was not used to enrich the introspected datamodel with previous manual changes."
                            .into(),
                        affected: serde_json::Value::Null,
                    });
                        warnings
                    }
                    _ => introspection_result.warnings,
                };

                let result = IntrospectionResultOutput {
                    datamodel: datamodel::render_datamodel_and_config_to_string(
                        &introspection_result.datamodel,
                        &config,
                    )
                    .map_err(Error::from)?,
                    warnings,
                    version: introspection_result.version,
                };

                Ok(result)
            }
            Err(e) => Err(render_jsonrpc_error(Error::from(e))),
        }
    }

    pub async fn list_databases_internal(schema: String) -> RpcResult<Vec<String>> {
        let connector = RpcImpl::load_connector(&schema).await?;
        Ok(connector.list_databases().await.map_err(Error::from)?)
    }

    pub async fn get_database_description(schema: String) -> RpcResult<String> {
        let connector = RpcImpl::load_connector(&schema).await?;
        Ok(connector.get_database_description().await.map_err(Error::from)?)
    }

    pub async fn get_database_metadata_internal(schema: String) -> RpcResult<DatabaseMetadata> {
        let connector = RpcImpl::load_connector(&schema).await?;
        Ok(connector.get_metadata().await.map_err(Error::from)?)
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
