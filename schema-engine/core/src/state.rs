//! A container to manage 0 or more schema connectors, based on request contents.
//!
//! Why this rather than using connectors directly? We must be able to use the schema engine
//! without a valid schema or database connection for commands like createDatabase and diff.

use crate::{commands, parse_configuration_multi, CoreError, CoreResult, GenericApi, SchemaContainerExt};
use enumflags2::BitFlags;
use json_rpc::types::*;
use psl::{parser_database::SourceFile, PreviewFeature};
use schema_connector::{ConnectorError, ConnectorHost, IntrospectionResult, Namespaces, SchemaConnector};
use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::{mpsc, Mutex};
use tracing_futures::Instrument;

/// The container for the state of the schema engine. It can contain one or more connectors
/// corresponding to a database to be reached or that we are already connected to.
///
/// The general mechanism is that we match a single url or prisma schema to a single connector in
/// `connectors`. Each connector has its own async task, and communicates with the core through
/// channels. That ensures that each connector is handling requests one at a time to avoid
/// synchronization issues. You can think of it in terms of the actor model.
pub(crate) struct EngineState {
    initial_datamodel: Option<psl::ValidatedSchema>,
    host: Arc<dyn ConnectorHost>,
    // A map from either:
    //
    // - a connection string / url
    // - a full schema
    //
    // To a channel leading to a spawned MigrationConnector.
    connectors: Mutex<HashMap<ConnectorRequestType, mpsc::Sender<ErasedConnectorRequest>>>,
}

impl EngineState {
    fn get_url_from_schemas(&self, container: &SchemasWithConfigDir) -> CoreResult<String> {
        let sources = container.to_psl_input();
        let (datasource, url, _, _) = parse_configuration_multi(&sources)?;

        Ok(psl::set_config_dir(
            datasource.active_connector.flavour(),
            std::path::Path::new(&container.config_dir),
            &url,
        )
        .into_owned())
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
enum ConnectorRequestType {
    Schema(Vec<(String, SourceFile)>),
    Url(String),
}

/// A request from the core to a connector, in the form of an async closure.
type ConnectorRequest<O> = Box<
    dyn for<'c> FnOnce(&'c mut dyn SchemaConnector) -> Pin<Box<dyn Future<Output = CoreResult<O>> + Send + 'c>> + Send,
>;

/// Same as ConnectorRequest, but with the return type erased with a channel.
type ErasedConnectorRequest = Box<
    dyn for<'c> FnOnce(&'c mut dyn SchemaConnector) -> Pin<Box<dyn Future<Output = ()> + Send + 'c>> + Send + 'static,
>;

impl EngineState {
    pub(crate) fn new(
        initial_datamodels: Option<Vec<(String, SourceFile)>>,
        host: Option<Arc<dyn ConnectorHost>>,
    ) -> Self {
        EngineState {
            initial_datamodel: initial_datamodels.as_deref().map(psl::validate_multi_file),
            host: host.unwrap_or_else(|| Arc::new(schema_connector::EmptyHost)),
            connectors: Default::default(),
        }
    }

    fn namespaces(&self) -> Option<Namespaces> {
        self.initial_datamodel
            .as_ref()
            .and_then(|schema| schema.configuration.datasources.first())
            .and_then(|ds| {
                let mut names = ds.namespaces.iter().map(|(ns, _)| ns.to_owned()).collect();
                Namespaces::from_vec(&mut names)
            })
    }

    async fn with_connector_for_schema<O: Send + 'static>(
        &self,
        schemas: Vec<(String, SourceFile)>,
        config_dir: Option<&Path>,
        f: ConnectorRequest<O>,
    ) -> CoreResult<O> {
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel::<CoreResult<O>>();
        let erased: ErasedConnectorRequest = Box::new(move |connector| {
            Box::pin(async move {
                let output = f(connector).await;
                response_sender
                    .send(output)
                    .map_err(|_| ())
                    .expect("failed to send back response in schema-engine state");
            })
        });

        let mut connectors = self.connectors.lock().await;

        match connectors.get(&ConnectorRequestType::Schema(schemas.clone())) {
            Some(request_sender) => match request_sender.send(erased).await {
                Ok(()) => (),
                Err(_) => return Err(ConnectorError::from_msg("tokio mpsc send error".to_owned())),
            },
            None => {
                let mut connector = crate::schema_to_connector(&schemas, config_dir)?;

                connector.set_host(self.host.clone());
                let (erased_sender, mut erased_receiver) = mpsc::channel::<ErasedConnectorRequest>(12);
                tokio::spawn(async move {
                    while let Some(req) = erased_receiver.recv().await {
                        req(connector.as_mut()).await;
                    }
                });
                match erased_sender.send(erased).await {
                    Ok(()) => (),
                    Err(_) => return Err(ConnectorError::from_msg("erased sender send error".to_owned())),
                };
                connectors.insert(ConnectorRequestType::Schema(schemas), erased_sender);
            }
        }

        response_receiver.await.expect("receiver boomed")
    }

    async fn with_connector_for_url<O: Send + 'static>(&self, url: String, f: ConnectorRequest<O>) -> CoreResult<O> {
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel::<CoreResult<O>>();
        let erased: ErasedConnectorRequest = Box::new(move |connector| {
            Box::pin(async move {
                let output = f(connector).await;
                response_sender
                    .send(output)
                    .map_err(|_| ())
                    .expect("failed to send back response in schema-engine state");
            })
        });

        let mut connectors = self.connectors.lock().await;
        match connectors.get(&ConnectorRequestType::Url(url.clone())) {
            Some(request_sender) => match request_sender.send(erased).await {
                Ok(()) => (),
                Err(_) => return Err(ConnectorError::from_msg("tokio mpsc send error".to_owned())),
            },
            None => {
                let mut connector = crate::connector_for_connection_string(url.clone(), None, BitFlags::default())?;
                connector.set_host(self.host.clone());

                let (erased_sender, mut erased_receiver) = mpsc::channel::<ErasedConnectorRequest>(12);
                tokio::spawn(async move {
                    while let Some(req) = erased_receiver.recv().await {
                        req(connector.as_mut()).await;
                    }
                });
                match erased_sender.send(erased).await {
                    Ok(()) => (),
                    Err(_) => return Err(ConnectorError::from_msg("erased sender send error".to_owned())),
                };

                connectors.insert(ConnectorRequestType::Url(url), erased_sender);
            }
        }

        response_receiver.await.expect("receiver boomed")
    }

    async fn with_connector_from_datasource_param<O: Send + 'static>(
        &self,
        param: DatasourceParam,
        f: ConnectorRequest<O>,
    ) -> CoreResult<O> {
        match param {
            DatasourceParam::ConnectionString(UrlContainer { url }) => self.with_connector_for_url(url, f).await,
            DatasourceParam::Schema(schemas) => self.with_connector_for_schema(schemas.to_psl_input(), None, f).await,
        }
    }

    async fn with_default_connector<O>(&self, f: ConnectorRequest<O>) -> CoreResult<O>
    where
        O: Sized + Send + 'static,
    {
        let schema = if let Some(initial_datamodel) = &self.initial_datamodel {
            initial_datamodel
        } else {
            return Err(ConnectorError::from_msg("Missing --datamodels".to_owned()));
        };

        let schemas = schema
            .db
            .iter_file_sources()
            .map(|(name, content)| (name.to_string(), content.clone()))
            .collect::<Vec<_>>();

        self.with_connector_for_schema(schemas, None, f).await
    }
}

#[async_trait::async_trait]
impl GenericApi for EngineState {
    async fn version(&self, params: Option<GetDatabaseVersionInput>) -> CoreResult<String> {
        let f: ConnectorRequest<String> = Box::new(|connector| connector.version());

        match params {
            Some(params) => self.with_connector_from_datasource_param(params.datasource, f).await,
            None => self.with_default_connector(f).await,
        }
    }

    async fn apply_migrations(&self, input: ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput> {
        let namespaces = self.namespaces();

        self.with_default_connector(Box::new(move |connector| {
            Box::pin(
                ::commands::apply_migrations(input, connector, namespaces)
                    .instrument(tracing::info_span!("ApplyMigrations")),
            )
        }))
        .await
    }

    async fn create_database(&self, params: CreateDatabaseParams) -> CoreResult<CreateDatabaseResult> {
        self.with_connector_from_datasource_param(
            params.datasource,
            Box::new(|connector| {
                Box::pin(async move {
                    let database_name = SchemaConnector::create_database(connector).await?;
                    Ok(CreateDatabaseResult { database_name })
                })
            }),
        )
        .await
    }

    async fn create_migration(&self, input: CreateMigrationInput) -> CoreResult<CreateMigrationOutput> {
        self.with_default_connector(Box::new(move |connector| {
            let span = tracing::info_span!(
                "CreateMigration",
                migration_name = input.migration_name.as_str(),
                draft = input.draft,
            );
            Box::pin(commands::create_migration(input, connector).instrument(span))
        }))
        .await
    }

    async fn db_execute(&self, params: DbExecuteParams) -> CoreResult<()> {
        let url: String = match &params.datasource_type {
            DbExecuteDatasourceType::Url(UrlContainer { url }) => url.clone(),
            DbExecuteDatasourceType::Schema(schemas) => self.get_url_from_schemas(schemas)?,
        };

        self.with_connector_for_url(url, Box::new(move |connector| connector.db_execute(params.script)))
            .await
    }

    async fn debug_panic(&self) -> CoreResult<()> {
        panic!("This is the debugPanic artificial panic")
    }

    async fn dev_diagnostic(&self, input: DevDiagnosticInput) -> CoreResult<DevDiagnosticOutput> {
        let namespaces = self.namespaces();
        self.with_default_connector(Box::new(move |connector| {
            Box::pin(async move {
                commands::dev_diagnostic(input, namespaces, connector)
                    .instrument(tracing::info_span!("DevDiagnostic"))
                    .await
            })
        }))
        .await
    }

    async fn diff(&self, params: DiffParams) -> CoreResult<DiffResult> {
        crate::commands::diff(params, self.host.clone()).await
    }

    async fn drop_database(&self, url: String) -> CoreResult<()> {
        self.with_connector_for_url(url, Box::new(|connector| SchemaConnector::drop_database(connector)))
            .await
    }

    async fn diagnose_migration_history(
        &self,
        input: commands::DiagnoseMigrationHistoryInput,
    ) -> CoreResult<commands::DiagnoseMigrationHistoryOutput> {
        let namespaces = self.namespaces();
        self.with_default_connector(Box::new(move |connector| {
            Box::pin(async move {
                commands::diagnose_migration_history(input, namespaces, connector)
                    .instrument(tracing::info_span!("DiagnoseMigrationHistory"))
                    .await
            })
        }))
        .await
    }

    async fn ensure_connection_validity(
        &self,
        params: EnsureConnectionValidityParams,
    ) -> CoreResult<EnsureConnectionValidityResult> {
        self.with_connector_from_datasource_param(
            params.datasource,
            Box::new(|connector| {
                Box::pin(async move {
                    SchemaConnector::ensure_connection_validity(connector).await?;
                    Ok(EnsureConnectionValidityResult {})
                })
            }),
        )
        .await
    }

    async fn evaluate_data_loss(&self, input: EvaluateDataLossInput) -> CoreResult<EvaluateDataLossOutput> {
        self.with_default_connector(Box::new(|connector| {
            Box::pin(commands::evaluate_data_loss(input, connector).instrument(tracing::info_span!("EvaluateDataLoss")))
        }))
        .await
    }

    async fn introspect(&self, params: IntrospectParams) -> CoreResult<IntrospectResult> {
        tracing::info!("{:?}", params.schema);
        let source_files = params.schema.to_psl_input();

        let has_some_namespaces = params.namespaces.is_some();
        let composite_type_depth = From::from(params.composite_type_depth);

        let ctx = if params.force {
            let previous_schema = psl::validate_multi_file(&source_files);

            schema_connector::IntrospectionContext::new_config_only(
                previous_schema,
                composite_type_depth,
                params.namespaces,
                PathBuf::new().join(&params.base_directory_path),
            )
        } else {
            let previous_schema =
                psl::parse_schema_multi(&source_files).map_err(ConnectorError::new_schema_parser_error)?;

            schema_connector::IntrospectionContext::new(
                previous_schema,
                composite_type_depth,
                params.namespaces,
                PathBuf::new().join(&params.base_directory_path),
            )
        };

        if !ctx
            .configuration()
            .preview_features()
            .contains(PreviewFeature::MultiSchema)
            && has_some_namespaces
        {
            let msg =
                "The preview feature `multiSchema` must be enabled before using --schemas command line parameter.";

            return Err(CoreError::from_msg(msg.to_string()));
        }

        self.with_connector_for_schema(
            source_files,
            None,
            Box::new(move |connector| {
                Box::pin(async move {
                    let IntrospectionResult {
                        datamodels,
                        views,
                        warnings,
                        is_empty,
                    } = connector.introspect(&ctx).await?;

                    if is_empty {
                        Err(ConnectorError::into_introspection_result_empty_error())
                    } else {
                        let views = views.map(|v| {
                            v.into_iter()
                                .map(|view| IntrospectionView {
                                    schema: view.schema,
                                    name: view.name,
                                    definition: view.definition,
                                })
                                .collect()
                        });

                        Ok(IntrospectResult {
                            schema: SchemasContainer {
                                files: datamodels
                                    .into_iter()
                                    .map(|(path, content)| SchemaContainer { path, content })
                                    .collect(),
                            },
                            views,
                            warnings,
                        })
                    }
                })
            }),
        )
        .await
    }

    async fn introspect_sql(&self, params: IntrospectSqlParams) -> CoreResult<IntrospectSqlResult> {
        self.with_connector_for_url(
            params.url.clone(),
            Box::new(move |conn| {
                Box::pin(async move {
                    let res = crate::commands::introspect_sql(params, conn).await?;

                    Ok(IntrospectSqlResult {
                        queries: res
                            .queries
                            .into_iter()
                            .map(|q| SqlQueryOutput {
                                name: q.name,
                                source: q.source,
                                documentation: q.documentation,
                                parameters: q
                                    .parameters
                                    .into_iter()
                                    .map(|p| SqlQueryParameterOutput {
                                        name: p.name,
                                        typ: p.typ,
                                        documentation: p.documentation,
                                        nullable: p.nullable,
                                    })
                                    .collect(),
                                result_columns: q
                                    .result_columns
                                    .into_iter()
                                    .map(|c| SqlQueryColumnOutput {
                                        name: c.name,
                                        typ: c.typ,
                                        nullable: c.nullable,
                                    })
                                    .collect(),
                            })
                            .collect(),
                    })
                })
            }),
        )
        .await
    }

    async fn mark_migration_applied(&self, input: MarkMigrationAppliedInput) -> CoreResult<MarkMigrationAppliedOutput> {
        self.with_default_connector(Box::new(move |connector| {
            let span = tracing::info_span!("MarkMigrationApplied", migration_name = input.migration_name.as_str());
            Box::pin(commands::mark_migration_applied(input, connector).instrument(span))
        }))
        .await
    }

    async fn mark_migration_rolled_back(
        &self,
        input: MarkMigrationRolledBackInput,
    ) -> CoreResult<MarkMigrationRolledBackOutput> {
        self.with_default_connector(Box::new(move |connector| {
            let span = tracing::info_span!(
                "MarkMigrationRolledBack",
                migration_name = input.migration_name.as_str()
            );
            Box::pin(commands::mark_migration_rolled_back(input, connector).instrument(span))
        }))
        .await
    }

    async fn reset(&self) -> CoreResult<()> {
        tracing::debug!("Resetting the database.");
        let namespaces = self.namespaces();
        self.with_default_connector(Box::new(move |connector| {
            Box::pin(SchemaConnector::reset(connector, false, namespaces).instrument(tracing::info_span!("Reset")))
        }))
        .await?;
        Ok(())
    }

    async fn schema_push(&self, input: SchemaPushInput) -> CoreResult<SchemaPushOutput> {
        self.with_default_connector(Box::new(move |connector| {
            Box::pin(commands::schema_push(input, connector).instrument(tracing::info_span!("SchemaPush")))
        }))
        .await
    }
}
