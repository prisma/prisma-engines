use crate::error::Error;
use datamodel::common::preview_features::PreviewFeature;
use datamodel::{Configuration, Datamodel};
use introspection_connector::{ConnectorResult, DatabaseMetadata, IntrospectionConnector, IntrospectionResultOutput};
use jsonrpc_core::BoxFuture;
use jsonrpc_derive::rpc;
use serde_derive::*;
use sql_introspection_connector::SqlIntrospectionConnector;

type RpcError = jsonrpc_core::Error;
type RpcResult<T> = Result<T, RpcError>;
type RpcFutureResult<T> = BoxFuture<RpcResult<T>>;

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

    #[rpc(name = "debugPanic")]
    fn debug_panic(&self) -> RpcFutureResult<()>;
}

pub struct RpcImpl;

impl Rpc for RpcImpl {
    fn list_databases(&self, input: IntrospectionInput) -> RpcFutureResult<Vec<String>> {
        Box::pin(Self::list_databases_internal(input.schema))
    }

    fn get_database_metadata(&self, input: IntrospectionInput) -> RpcFutureResult<DatabaseMetadata> {
        Box::pin(Self::get_database_metadata_internal(input.schema))
    }

    fn get_database_description(&self, input: IntrospectionInput) -> RpcFutureResult<String> {
        Box::pin(Self::get_database_description_internal(input.schema))
    }

    fn get_database_version(&self, input: IntrospectionInput) -> RpcFutureResult<String> {
        Box::pin(Self::get_database_version_internal(input.schema))
    }

    fn introspect(&self, input: IntrospectionInput) -> RpcFutureResult<IntrospectionResultOutput> {
        Box::pin(Self::introspect_internal(input.schema, input.force))
    }

    fn debug_panic(&self) -> RpcFutureResult<()> {
        Box::pin(Self::debug_panic())
    }
}

impl RpcImpl {
    async fn load_connector(schema: &str) -> Result<(Configuration, String, Box<dyn IntrospectionConnector>), Error> {
        let config = datamodel::parse_configuration(&schema)
            .map_err(|diagnostics| Error::DatamodelError(diagnostics.to_pretty_string("schema.prisma", schema)))?;

        let preview_features: Vec<PreviewFeature> = config.subject.preview_features().map(|x| x.to_owned()).collect();

        let url = config
            .subject
            .datasources
            .first()
            .ok_or_else(|| Error::Generic("There is no datasource in the schema.".into()))?
            .load_url(|key| std::env::var(key).ok())
            .map_err(|diagnostics| Error::DatamodelError(diagnostics.to_pretty_string("schema.prisma", schema)))?;

        Ok((
            config.subject,
            url.clone(),
            Box::new(SqlIntrospectionConnector::new(&url, preview_features).await?),
        ))
    }

    pub async fn catch<O>(fut: impl std::future::Future<Output = ConnectorResult<O>>) -> RpcResult<O> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(e) => Err(RpcError::from(Error::from(e))),
        }
    }

    pub async fn introspect_internal(schema: String, force: bool) -> RpcResult<IntrospectionResultOutput> {
        let (config, url, connector) = RpcImpl::load_connector(&schema).await?; //todo
        let (config2, _, _) = RpcImpl::load_connector(&schema).await?;

        let input_data_model = if !force {
            Self::parse_datamodel(&schema)?
        } else {
            Datamodel::new()
        };

        let first_source = config2.datasources.into_iter().next().unwrap();

        let result = match connector
            .introspect(&input_data_model, first_source.name, first_source.active_connector)
            .await
        {
            Ok(introspection_result) => {
                if introspection_result.data_model.is_empty() {
                    Err(Error::IntrospectionResultEmpty(url.to_string()))
                } else {
                    Ok(IntrospectionResultOutput {
                        datamodel: datamodel::render_datamodel_and_config_to_string(
                            &introspection_result.data_model,
                            &config,
                        ),
                        warnings: introspection_result.warnings,
                        version: introspection_result.version,
                    })
                }
            }
            Err(e) => Err(Error::from(e)),
        };

        result.map_err(RpcError::from)
    }

    /// This function parses the provided schema and returns the contained Datamodel.
    pub fn parse_datamodel(schema: &str) -> RpcResult<Datamodel> {
        let final_dm = datamodel::parse_datamodel(&schema)
            .map(|d| d.subject)
            .map_err(|err| Error::DatamodelError(err.to_pretty_string("schema.prisma", &schema)))?;

        Ok(final_dm)
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

    pub async fn debug_panic() -> RpcResult<()> {
        panic!("This is the debugPanic artificial panic")
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
