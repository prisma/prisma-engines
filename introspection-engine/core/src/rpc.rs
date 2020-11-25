use crate::command_error::CommandError;
use crate::error::Error;
use datamodel::configuration::preview_features::PreviewFeatures;
use datamodel::{Configuration, Datamodel, FieldArity};
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

    #[rpc(name = "debugPanic")]
    fn debug_panic(&self) -> RpcFutureResult<()>;
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

    fn debug_panic(&self) -> RpcFutureResult<()> {
        Box::new(Self::debug_panic().boxed().compat())
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
            .subject
            .datasources
            .first()
            .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?
            .url()
            .to_owned()
            .value;

        Ok((
            config.subject,
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
            Self::parse_datamodel(&schema, &config)?
        } else {
            Datamodel::new()
        };

        let native_types = match datamodel::parse_configuration(&schema) {
            Ok(config) => config
                .subject
                .generators
                .iter()
                .any(|g| g.has_preview_feature("nativeTypes")),
            Err(_) => false,
        };

        let result = match connector.introspect(&input_data_model, native_types).await {
            Ok(introspection_result) => {
                if introspection_result.data_model.is_empty() {
                    Err(Error::from(CommandError::IntrospectionResultEmpty(url.to_string())))
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
    /// In this processes it applies a patch to turn virtual relation fields that are required into optional instead.
    /// The reason for this is that we introduced a breaking change to disallow required virtual relation fields.
    /// With this patch we can tell users to simply run `prisma introspect` to fix their schema.
    fn parse_datamodel(schema: &str, config: &Configuration) -> RpcResult<Datamodel> {
        // 1. Parse the schema without any validations & standardisations. A required virtual relation field would fail validation as it is forbidden.
        let mut dm_that_needs_fixing = datamodel::parse_datamodel_without_validation(&schema).map_err(|err| {
            Error::from(CommandError::ReceivedBadDatamodel(
                err.to_pretty_string("schema.prisma", &schema),
            ))
        })?;

        // 2. Turn all virtual relation fields that are required into optional ones.
        let cloned_dm = dm_that_needs_fixing.clone();
        for model in dm_that_needs_fixing.models.iter_mut() {
            for relation_field in model.relation_fields_mut() {
                // if there's no related field we don't know enough to do this patch
                if let Some(related_field) = cloned_dm.find_related_field(relation_field) {
                    let is_required_virtual_relation_field =
                        relation_field.arity.is_required() && relation_field.is_virtual();

                    // we only do this if the related field is not virtual
                    // if the related field is virtual as well this means standardisation has not run yet and we don't know which one is virtual for sure
                    if is_required_virtual_relation_field && !related_field.is_virtual() {
                        relation_field.arity = FieldArity::Optional;
                    }
                }
            }
        }

        // 3. Render the datamodel and then parse it. This makes sure the validations & standardisations have been run.
        let rendered_datamodel = datamodel::render_datamodel_and_config_to_string(&dm_that_needs_fixing, &config);
        datamodel::parse_datamodel(&rendered_datamodel)
            .map(|d| d.subject)
            .map_err(|err| {
                Error::from(CommandError::ReceivedBadDatamodel(
                    err.to_pretty_string("schema.prisma", &schema),
                ))
            })?;

        Ok(dm_that_needs_fixing)
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
