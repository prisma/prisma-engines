//! A container to manage 0 or more migration connectors, based on request contents.
//!
//! Why this rather than using connectors directly? We must be able to use the migration engine
//! without a valid schema or database connection for commands like createDatabase and diff.

use crate::{api::GenericApi, commands, json_rpc::types::*, CoreResult};
use enumflags2::BitFlags;
use migration_connector::{ConnectorError, ConnectorHost, MigrationConnector, Namespaces};
use psl::parser_database::SourceFile;
use std::{collections::HashMap, future::Future, path::Path, pin::Pin, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use tracing_futures::Instrument;

/// The container for the state of the migration engine. It can contain one or more connectors
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
    connectors: Mutex<HashMap<String, mpsc::Sender<ErasedConnectorRequest>>>,
}

/// A request from the core to a connector, in the form of an async closure.
type ConnectorRequest<O> = Box<
    dyn for<'c> FnOnce(&'c mut dyn MigrationConnector) -> Pin<Box<dyn Future<Output = CoreResult<O>> + Send + 'c>>
        + Send,
>;

/// Same as ConnectorRequest, but with the return type erased with a channel.
type ErasedConnectorRequest = Box<
    dyn for<'c> FnOnce(&'c mut dyn MigrationConnector) -> Pin<Box<dyn Future<Output = ()> + Send + 'c>>
        + Send
        + 'static,
>;

impl EngineState {
    pub(crate) fn new(initial_datamodel: Option<String>, host: Option<Arc<dyn ConnectorHost>>) -> Self {
        EngineState {
            initial_datamodel: initial_datamodel.map(|s| psl::validate(s.into())),
            host: host.unwrap_or_else(|| Arc::new(migration_connector::EmptyHost)),
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

    async fn with_connector_from_schema_path<O: Send + 'static>(
        &self,
        path: &str,
        f: ConnectorRequest<O>,
    ) -> CoreResult<O> {
        let config_dir = std::path::Path::new(path).parent();
        let schema = std::fs::read_to_string(path)
            .map_err(|err| ConnectorError::from_source(err, "Falied to read Prisma schema."))?;
        self.with_connector_for_schema(&schema, config_dir, f).await
    }

    async fn with_connector_for_schema<O: Send + 'static>(
        &self,
        schema: &str,
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
                    .expect("failed to send back response in migration-engine state");
            })
        });

        let mut connectors = self.connectors.lock().await;
        match connectors.get(schema) {
            Some(request_sender) => match request_sender.send(erased).await {
                Ok(()) => (),
                Err(_) => return Err(ConnectorError::from_msg("tokio mpsc send error".to_owned())),
            },
            None => {
                let mut connector = crate::schema_to_connector(schema, config_dir)?;
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
                connectors.insert(schema.to_owned(), erased_sender);
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
                    .expect("failed to send back response in migration-engine state");
            })
        });

        let mut connectors = self.connectors.lock().await;
        match connectors.get(&url) {
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
                connectors.insert(url, erased_sender);
            }
        }

        response_receiver.await.expect("receiver boomed")
    }

    async fn with_connector_from_datasource_param<O: Send + 'static>(
        &self,
        param: &DatasourceParam,
        f: ConnectorRequest<O>,
    ) -> CoreResult<O> {
        match param {
            DatasourceParam::ConnectionString(UrlContainer { url }) => {
                self.with_connector_for_url(url.clone(), f).await
            }
            DatasourceParam::SchemaPath(PathContainer { path }) => self.with_connector_from_schema_path(path, f).await,
            DatasourceParam::SchemaString(SchemaContainer { schema }) => {
                self.with_connector_for_schema(schema, None, f).await
            }
        }
    }

    async fn with_default_connector<O: Send + 'static>(&self, f: ConnectorRequest<O>) -> CoreResult<O>
    where
        O: Sized + Send + 'static,
    {
        let schema = if let Some(initial_datamodel) = &self.initial_datamodel {
            initial_datamodel
        } else {
            return Err(ConnectorError::from_msg("Missing --datamodel".to_owned()));
        };

        self.with_connector_for_schema(schema.db.source(), None, f).await
    }
}

#[async_trait::async_trait]
impl GenericApi for EngineState {
    async fn version(&self) -> CoreResult<String> {
        self.with_default_connector(Box::new(|connector| connector.version()))
            .await
    }

    async fn apply_migrations(&self, input: ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput> {
        self.with_default_connector(Box::new(move |connector| {
            Box::pin(commands::apply_migrations(input, connector).instrument(tracing::info_span!("ApplyMigrations")))
        }))
        .await
    }

    async fn create_database(&self, params: CreateDatabaseParams) -> CoreResult<CreateDatabaseResult> {
        self.with_connector_from_datasource_param(
            &params.datasource,
            Box::new(|connector| {
                Box::pin(async move {
                    let database_name = MigrationConnector::create_database(connector).await?;
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
        use std::io::Read;

        let url: String = match &params.datasource_type {
            DbExecuteDatasourceType::Url(UrlContainer { url }) => url.clone(),
            DbExecuteDatasourceType::Schema(SchemaContainer { schema: file_path }) => {
                let mut schema_file = std::fs::File::open(file_path)
                    .map_err(|err| ConnectorError::from_source(err, "Opening Prisma schema file."))?;
                let mut schema_string = String::new();
                schema_file
                    .read_to_string(&mut schema_string)
                    .map_err(|err| ConnectorError::from_source(err, "Reading Prisma schema file."))?;
                let (datasource, url, _, _) = crate::parse_configuration(&schema_string)?;
                std::path::Path::new(file_path)
                    .parent()
                    .map(|config_dir| {
                        datasource
                            .active_connector
                            .set_config_dir(config_dir, &url)
                            .into_owned()
                    })
                    .unwrap_or(url)
            }
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
        self.with_connector_for_url(url, Box::new(|connector| MigrationConnector::drop_database(connector)))
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
            &params.datasource,
            Box::new(|connector| {
                Box::pin(async move {
                    MigrationConnector::ensure_connection_validity(connector).await?;
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
        let source_file = SourceFile::new_allocated(Arc::from(params.schema.clone().into_boxed_str()));
        let schema = psl::parse_schema(source_file).map_err(ConnectorError::new_schema_parser_error)?;
        self.with_connector_for_schema(
            &params.schema,
            None,
            Box::new(move |connector| {
                let composite_type_depth = From::from(params.composite_type_depth);
                let ctx = migration_connector::IntrospectionContext::new(schema, composite_type_depth);
                Box::pin(async move {
                    // TODO(MultiSchema): Grab namespaces from introspect params?
                    let result = connector.introspect(&ctx, None).await?;

                    Ok(IntrospectResult {
                        datamodel: result.data_model,
                        version: format!("{:?}", result.version),
                        warnings: result
                            .warnings
                            .into_iter()
                            .map(|warning| crate::json_rpc::types::IntrospectionWarning {
                                code: warning.code as u32,
                                message: warning.message,
                                affected: warning.affected,
                            })
                            .collect(),
                    })
                })
            }),
        )
        .await
    }

    async fn list_migration_directories(
        &self,
        input: ListMigrationDirectoriesInput,
    ) -> CoreResult<ListMigrationDirectoriesOutput> {
        let migrations_from_filesystem =
            migration_connector::migrations_directory::list_migrations(Path::new(&input.migrations_directory_path))?;

        let migrations = migrations_from_filesystem
            .iter()
            .map(|migration| migration.migration_name().to_string())
            .collect();

        Ok(ListMigrationDirectoriesOutput { migrations })
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
            Box::pin(MigrationConnector::reset(connector, false, namespaces).instrument(tracing::info_span!("Reset")))
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
