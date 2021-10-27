use crate::error::Error;
use datamodel::{Configuration, Datamodel};
use introspection_connector::{
    CompositeTypeDepth, ConnectorResult, DatabaseMetadata, IntrospectionConnector, IntrospectionContext,
    IntrospectionResultOutput,
};
use jsonrpc_core::BoxFuture;
use jsonrpc_derive::rpc;
use mongodb_introspection_connector::MongoDbIntrospectionConnector;
use serde_derive::*;
use sql_introspection_connector::SqlIntrospectionConnector;

type RpcError = jsonrpc_core::Error;
type RpcResult<T> = Result<T, RpcError>;
type RpcFutureResult<T> = BoxFuture<RpcResult<T>>;

#[rpc(server)]
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
        Box::pin(Self::introspect_internal(
            input.schema,
            input.force,
            CompositeTypeDepth::from(input.composite_type_depth.unwrap_or(0)),
        ))
    }

    fn debug_panic(&self) -> RpcFutureResult<()> {
        Box::pin(Self::debug_panic())
    }
}

impl RpcImpl {
    async fn load_connector(schema: &str) -> Result<(Configuration, String, Box<dyn IntrospectionConnector>), Error> {
        let config = datamodel::parse_configuration(schema)
            .map_err(|diagnostics| Error::DatamodelError(diagnostics.to_pretty_string("schema.prisma", schema)))?;

        let preview_features = config.subject.preview_features();

        let connection_string = config
            .subject
            .datasources
            .first()
            .ok_or_else(|| Error::Generic("There is no datasource in the schema.".into()))?
            .load_url(|key| std::env::var(key).ok())
            .map_err(|diagnostics| Error::DatamodelError(diagnostics.to_pretty_string("schema.prisma", schema)))?;

        let connector: Box<dyn IntrospectionConnector> = if connection_string.starts_with("mongo") {
            Box::new(MongoDbIntrospectionConnector::new(&connection_string).await?)
        } else {
            Box::new(SqlIntrospectionConnector::new(&connection_string, preview_features).await?)
        };

        Ok((config.subject, connection_string.clone(), connector))
    }

    pub async fn catch<O>(fut: impl std::future::Future<Output = ConnectorResult<O>>) -> RpcResult<O> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(e) => Err(RpcError::from(Error::from(e))),
        }
    }

    pub async fn introspect_internal(
        schema: String,
        force: bool,
        composite_type_depth: CompositeTypeDepth,
    ) -> RpcResult<IntrospectionResultOutput> {
        let (config, url, connector) = RpcImpl::load_connector(&schema).await?;

        let input_data_model = if !force {
            Self::parse_datamodel(&schema)?
        } else {
            Datamodel::new()
        };

        let (config2, _, _) = RpcImpl::load_connector(&schema).await?;

        let ctx = IntrospectionContext {
            preview_features: config2.preview_features(),
            source: config2.datasources.into_iter().next().unwrap(),
            composite_type_depth,
        };

        let result = match connector.introspect(&input_data_model, ctx).await {
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
        let final_dm = datamodel::parse_datamodel(schema)
            .map(|d| d.subject)
            .map_err(|err| Error::DatamodelError(err.to_pretty_string("schema.prisma", schema)))?;

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
#[serde(rename_all = "camelCase")]
pub struct IntrospectionInput {
    pub(crate) schema: String,
    #[serde(default = "default_false")]
    pub(crate) force: bool,
    #[serde(default)]
    pub(crate) composite_type_depth: Option<isize>,
}

fn default_false() -> bool {
    false
}
