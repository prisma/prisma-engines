//! A container to manage 0 or more migration connectors, based on request contents.
//!
//! Why: we must be able to use the migration engine without a valid schema or database connection
//! for commands like createDatabase and diff.

use crate::{api::GenericApi, commands, json_rpc::types::*, CoreResult};
use enumflags2::BitFlag;
use migration_connector::{ConnectorError, ConnectorHost, MigrationConnector};
use std::{collections::HashMap, future::Future, path::Path, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tracing_futures::Instrument;

/// The container for the state of the migration engine. It can contain one or more connectors.
pub(crate) struct EngineState {
    initial_datamodel: Option<String>,
    host: Arc<dyn ConnectorHost>,
    // A map from either:
    //
    // - a connection string / url
    // - a full schema
    //
    // To a MigrationConnector.
    connectors: Mutex<HashMap<String, Box<dyn MigrationConnector>>>,
}

impl EngineState {
    pub(crate) fn new(initial_datamodel: Option<String>, host: Option<Arc<dyn ConnectorHost>>) -> Self {
        EngineState {
            initial_datamodel,
            host: host.unwrap_or_else(|| Arc::new(migration_connector::EmptyHost)),
            connectors: Default::default(),
        }
    }

    async fn with_connector_from_schema_path<O>(
        &self,
        path: &str,
        f: impl for<'c> FnOnce(&'c dyn MigrationConnector) -> Pin<Box<dyn Future<Output = CoreResult<O>> + Send + 'c>>,
    ) -> CoreResult<O> {
        let schema = std::fs::read_to_string(path)
            .map_err(|err| ConnectorError::from_source(err, "Falied to read Prisma schema."))?;
        self.with_connector_for_schema(&schema, f).await
    }

    async fn with_connector_for_schema<O>(
        &self,
        schema: &str,
        f: impl for<'c> FnOnce(&'c dyn MigrationConnector) -> Pin<Box<dyn Future<Output = CoreResult<O>> + Send + 'c>>,
    ) -> CoreResult<O> {
        let mut connectors = self.connectors.lock().await;

        match connectors.get(schema) {
            Some(connector) => f(connector.as_ref()).await,
            None => {
                let mut connector = crate::schema_to_connector(schema)?;
                connector.set_host(self.host.clone());
                let output = f(connector.as_ref()).await?;
                connectors.insert(schema.to_owned(), connector);
                Ok(output)
            }
        }
    }

    async fn with_connector_for_url<O>(
        &self,
        url: String,
        f: impl for<'c> FnOnce(&'c dyn MigrationConnector) -> Pin<Box<dyn Future<Output = CoreResult<O>> + Send + 'c>>,
    ) -> CoreResult<O> {
        let mut connectors = self.connectors.lock().await;

        match connectors.get(&url) {
            Some(connector) => f(connector.as_ref()).await,
            None => {
                let mut connector = crate::connector_for_connection_string(url.clone(), None, BitFlag::empty())?;
                connector.set_host(self.host.clone());
                let output = f(connector.as_ref()).await?;
                connectors.insert(url, connector);
                Ok(output)
            }
        }
    }

    async fn with_connector_from_datasource_param<O>(
        &self,
        param: &DatasourceParam,
        f: impl for<'c> FnOnce(&'c dyn MigrationConnector) -> Pin<Box<dyn Future<Output = CoreResult<O>> + Send + 'c>>,
    ) -> CoreResult<O> {
        match param {
            DatasourceParam::ConnectionString(UrlContainer { url }) => {
                self.with_connector_for_url(url.clone(), f).await
            }
            DatasourceParam::SchemaPath(PathContainer { path }) => self.with_connector_from_schema_path(path, f).await,
            DatasourceParam::SchemaString(SchemaContainer { schema }) => {
                self.with_connector_for_schema(schema, f).await
            }
        }
    }

    async fn with_default_connector<O>(
        &self,
        f: impl for<'c> FnOnce(&'c dyn MigrationConnector) -> Pin<Box<dyn Future<Output = CoreResult<O>> + Send + 'c>>,
    ) -> CoreResult<O>
    where
        O: Sized + 'static,
    {
        let schema = if let Some(initial_datamodel) = &self.initial_datamodel {
            initial_datamodel
        } else {
            return Err(ConnectorError::from_msg("Missing --datamodel".to_owned()));
        };

        self.with_connector_for_schema(schema, f).await
    }
}

#[async_trait::async_trait]
impl GenericApi for EngineState {
    async fn version(&self) -> CoreResult<String> {
        self.with_default_connector(move |connector| connector.version()).await
    }

    async fn apply_migrations(&self, input: ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput> {
        self.with_default_connector(move |connector| {
            Box::pin(commands::apply_migrations(input, connector).instrument(tracing::info_span!("ApplyMigrations")))
        })
        .await
    }

    async fn create_database(&self, params: CreateDatabaseParams) -> CoreResult<CreateDatabaseResult> {
        self.with_connector_from_datasource_param(&params.datasource, |connector| {
            Box::pin(async move {
                let database_name = MigrationConnector::create_database(connector).await?;
                Ok(CreateDatabaseResult { database_name })
            })
        })
        .await
    }

    async fn create_migration(&self, input: CreateMigrationInput) -> CoreResult<CreateMigrationOutput> {
        self.with_default_connector(move |connector| {
            let span = tracing::info_span!(
                "CreateMigration",
                migration_name = input.migration_name.as_str(),
                draft = input.draft,
            );
            Box::pin(commands::create_migration(input, connector).instrument(span))
        })
        .await
    }

    async fn db_execute(&self, params: DbExecuteParams) -> CoreResult<()> {
        use std::io::Read;

        let url = match &params.datasource_type {
            DbExecuteDatasourceType::Url(UrlContainer { url }) => url.to_owned(),
            DbExecuteDatasourceType::Schema(SchemaContainer { schema: file_path }) => {
                let mut schema_file = std::fs::File::open(&file_path)
                    .map_err(|err| ConnectorError::from_source(err, "Opening Prisma schema file."))?;
                let mut schema_string = String::new();
                schema_file
                    .read_to_string(&mut schema_string)
                    .map_err(|err| ConnectorError::from_source(err, "Reading Prisma schema file."))?;
                let (_, url, _, _) = crate::parse_configuration(&schema_string)?;
                url
            }
        };

        self.with_connector_for_url(url.clone(), move |connector| connector.db_execute(url, params.script))
            .await
    }

    async fn debug_panic(&self) -> CoreResult<()> {
        panic!("This is the debugPanic artificial panic")
    }

    async fn dev_diagnostic(&self, input: DevDiagnosticInput) -> CoreResult<DevDiagnosticOutput> {
        self.with_default_connector(|connector| {
            Box::pin(commands::dev_diagnostic(input, connector).instrument(tracing::info_span!("DevDiagnostic")))
        })
        .await
    }

    async fn diff(&self, params: DiffParams) -> CoreResult<DiffResult> {
        crate::commands::diff(params, self.host.clone()).await
    }

    async fn drop_database(&self, url: String) -> CoreResult<()> {
        self.with_connector_for_url(url, |connector| MigrationConnector::drop_database(connector))
            .await
    }

    async fn diagnose_migration_history(
        &self,
        input: commands::DiagnoseMigrationHistoryInput,
    ) -> CoreResult<commands::DiagnoseMigrationHistoryOutput> {
        self.with_default_connector(|connector| {
            Box::pin(
                commands::diagnose_migration_history(input, connector)
                    .instrument(tracing::info_span!("DiagnoseMigrationHistory")),
            )
        })
        .await
    }

    async fn ensure_connection_validity(
        &self,
        params: EnsureConnectionValidityParams,
    ) -> CoreResult<EnsureConnectionValidityResult> {
        self.with_connector_from_datasource_param(&params.datasource, |connector| {
            Box::pin(async move {
                MigrationConnector::ensure_connection_validity(connector).await?;
                Ok(EnsureConnectionValidityResult {})
            })
        })
        .await
    }

    async fn evaluate_data_loss(&self, input: EvaluateDataLossInput) -> CoreResult<EvaluateDataLossOutput> {
        self.with_default_connector(|connector| {
            Box::pin(commands::evaluate_data_loss(input, connector).instrument(tracing::info_span!("EvaluateDataLoss")))
        })
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
        self.with_default_connector(move |connector| {
            let span = tracing::info_span!("MarkMigrationApplied", migration_name = input.migration_name.as_str());
            Box::pin(commands::mark_migration_applied(input, connector).instrument(span))
        })
        .await
    }

    async fn mark_migration_rolled_back(
        &self,
        input: MarkMigrationRolledBackInput,
    ) -> CoreResult<MarkMigrationRolledBackOutput> {
        self.with_default_connector(move |connector| {
            let span = tracing::info_span!(
                "MarkMigrationRolledBack",
                migration_name = input.migration_name.as_str()
            );
            Box::pin(commands::mark_migration_rolled_back(input, connector).instrument(span))
        })
        .await
    }

    async fn reset(&self) -> CoreResult<()> {
        tracing::debug!("Resetting the database.");

        self.with_default_connector(move |connector| {
            Box::pin(MigrationConnector::reset(connector).instrument(tracing::info_span!("Reset")))
        })
        .await?;
        Ok(())
    }

    async fn schema_push(&self, input: SchemaPushInput) -> CoreResult<SchemaPushOutput> {
        self.with_default_connector(move |connector| {
            Box::pin(commands::schema_push(input, connector).instrument(tracing::info_span!("SchemaPush")))
        })
        .await
    }
}
