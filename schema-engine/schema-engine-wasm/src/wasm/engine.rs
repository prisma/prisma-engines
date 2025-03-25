#![allow(dead_code)]
#![allow(unused_variables)]

use commands::{
    schema_connector::{self, ConnectorError, IntrospectionResult, Namespaces, SchemaConnector},
    CoreError, SchemaContainerExt,
};
use driver_adapters::{adapter_factory_from_js, JsObject};
use json_rpc::types::*;
use psl::{parser_database::SourceFile, ConnectorRegistry, PreviewFeature};
use quaint::connector::ExternalConnectorFactory;
use sql_schema_connector::SqlSchemaConnector;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_futures::Instrument;
use wasm_bindgen::prelude::wasm_bindgen;

const CONNECTOR_REGISTRY: ConnectorRegistry<'_> = &[
    #[cfg(feature = "postgresql")]
    psl::builtin_connectors::POSTGRES,
    #[cfg(feature = "mysql")]
    psl::builtin_connectors::MYSQL,
    #[cfg(feature = "sqlite")]
    psl::builtin_connectors::SQLITE,
];

#[wasm_bindgen]
extern "C" {
    /// This function registers the reason for a Wasm panic via the
    /// JS function `globalThis.PRISMA_WASM_PANIC_REGISTRY.set_message()`
    #[wasm_bindgen(js_namespace = ["global", "PRISMA_WASM_PANIC_REGISTRY"], js_name = "set_message")]
    fn prisma_set_wasm_panic_message(s: &str);
}

/// Registers a singleton panic hook that will register the reason for the Wasm panic in JS.
/// Without this, the panic message would be lost: you'd see `RuntimeError: unreachable` message in JS,
/// with no reference to the Rust function and line that panicked.
/// This function should be manually called before any other public function in this module.
/// Note: no method is safe to call after a panic has occurred.
fn register_panic_hook() {
    use std::sync::Once;
    static SET_HOOK: Once = Once::new();

    SET_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let message = &info.to_string();
            prisma_set_wasm_panic_message(message);
        }));
    });
}

/// The main query engine used by JS
#[wasm_bindgen]
pub struct SchemaEngine {
    /// The adapter factory parsed from JS.
    adapter_factory: Arc<dyn ExternalConnectorFactory>,

    /// The SQL schema connector induced by the adapter.
    connector: SqlSchemaConnector,

    /// The inferred database namespaces (used for the `multiSchema` preview feature).
    namespaces: Option<Namespaces>,
}

// 1. One SchemaEngine object that reads 1 schema only and exposes methods that actually make use of such schema
// 2. A bunch of free functions (e.g., diff, version) that either don't rely on any schema,
//    or accept multiple schemas as input.

#[wasm_bindgen]
impl SchemaEngine {
    #[wasm_bindgen(constructor)]
    pub async fn new(adapter: JsObject) -> Result<SchemaEngine, wasm_bindgen::JsError> {
        register_panic_hook();

        let adapter_factory = Arc::new(adapter_factory_from_js(adapter));
        let adapter = Arc::new(adapter_factory.connect().await?);
        let connector = SqlSchemaConnector::new_from_external(adapter).await?;

        // TODO: retrieve the namespaces from JS, and forward them here.
        let namespaces: Option<Namespaces> = None;

        tracing::info!(git_hash = env!("GIT_HASH"), "Starting schema-engine-wasm");

        Ok(Self {
            adapter_factory,
            connector,
            namespaces,
        })
    }

    fn namespaces(&self) -> Option<Namespaces> {
        self.namespaces.clone()
    }

    /// Debugging method that only panics, for tests.
    #[wasm_bindgen(js_name = "debugPanic")]
    pub fn debug_panic(&self) {
        panic!("This is the debugPanic artificial panic")
    }

    /// Return the database version as a string.
    #[wasm_bindgen]
    pub async fn version(
        &mut self,
        // Note: custom params can currently be passed to the CLI's equivalent of this method
        // as a connection string or a list of PSL schemas.
        // This is incompatible with Driver Adapters.
        _params: Option<GetDatabaseVersionInput>,
    ) -> Result<Option<String>, wasm_bindgen::JsError> {
        let version = self.connector.version().await?;
        Ok(Some(version))
    }

    /// Apply all the unapplied migrations from the migrations folder.
    #[wasm_bindgen(js_name = "applyMigrations")]
    pub async fn apply_migrations(
        &mut self,
        input: ApplyMigrationsInput,
    ) -> Result<ApplyMigrationsOutput, wasm_bindgen::JsError> {
        let namespaces = self.namespaces();
        let result = commands::apply_migrations(input, &mut self.connector, namespaces)
            .instrument(tracing::info_span!("ApplyMigrations"))
            .await?;
        Ok(result)
    }

    /// Generate a new migration, based on the provided schema and existing migrations history.
    #[wasm_bindgen(js_name = "createMigration")]
    pub async fn create_migration(
        &mut self,
        input: CreateMigrationInput,
    ) -> Result<CreateMigrationOutput, wasm_bindgen::JsError> {
        let span = tracing::info_span!(
            "CreateMigration",
            migration_name = input.migration_name.as_str(),
            draft = input.draft,
        );
        let result = commands::create_migration(input, &mut self.connector)
            .instrument(span)
            .await?;
        Ok(result)
    }

    /// Send a raw command to the database.
    #[wasm_bindgen(js_name = "dbExecute")]
    pub async fn db_execute(&mut self, params: DbExecuteParams) -> Result<(), wasm_bindgen::JsError> {
        let result = self.connector.db_execute(params.script).await?;
        Ok(result)
    }

    /// Tells the CLI what to do in `migrate dev`.
    #[wasm_bindgen(js_name = "devDiagnostic")]
    pub async fn dev_diagnostic(
        &mut self,
        input: DevDiagnosticInput,
    ) -> Result<DevDiagnosticOutput, wasm_bindgen::JsError> {
        let namespaces = self.namespaces();
        let result = commands::dev_diagnostic(input, namespaces, &mut self.connector)
            .instrument(tracing::info_span!("DevDiagnostic"))
            .await?;
        Ok(result)
    }

    /// Create a migration between any two sources of database schemas.
    #[wasm_bindgen]
    pub async fn diff(&mut self, params: DiffParams) -> Result<DiffResult, wasm_bindgen::JsError> {
        let result = commands::diff(params, &mut self.connector)
            .instrument(tracing::info_span!("Diff"))
            .await?;
        Ok(result)
    }

    /// Looks at the migrations folder and the database, and returns a bunch of useful information.
    #[wasm_bindgen(js_name = "diagnoseMigrationHistory")]
    pub async fn diagnose_migration_history(
        &mut self,
        input: DiagnoseMigrationHistoryInput,
    ) -> Result<DiagnoseMigrationHistoryOutput, wasm_bindgen::JsError> {
        let namespaces = self.namespaces();
        let result: DiagnoseMigrationHistoryOutput =
            commands::diagnose_migration_history(input, namespaces, &mut self.connector)
                .instrument(tracing::info_span!("DiagnoseMigrationHistory"))
                .await?
                .into();
        Ok(result)
    }

    /// Make sure the connection to the database is established and valid.
    /// Connectors can choose to connect lazily, but this method should force
    /// them to connect.
    #[wasm_bindgen(js_name = "ensureConnectionValidity")]
    pub async fn ensure_connection_validity(
        &mut self,
        params: EnsureConnectionValidityParams,
    ) -> Result<EnsureConnectionValidityResult, wasm_bindgen::JsError> {
        SchemaConnector::ensure_connection_validity(&mut self.connector).await?;
        Ok(EnsureConnectionValidityResult {})
    }

    /// Evaluate the consequences of running the next migration we would generate, given the current state of a Prisma schema.
    #[wasm_bindgen(js_name = "evaluateDataLoss")]
    pub async fn evaluate_data_loss(
        &mut self,
        input: EvaluateDataLossInput,
    ) -> Result<EvaluateDataLossOutput, wasm_bindgen::JsError> {
        let result = commands::evaluate_data_loss(input, &mut self.connector)
            .instrument(tracing::info_span!("EvaluateDataLoss"))
            .await?;
        Ok(result)
    }

    /// Introspect the database schema.
    #[wasm_bindgen]
    pub async fn introspect(&mut self, params: IntrospectParams) -> Result<IntrospectResult, wasm_bindgen::JsError> {
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
            let previous_schema = psl::parse_schema_multi(&source_files)
                .map_err(|e| ConnectorError::new_schema_parser_error(e).into_js_error())?;

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

            return Err(CoreError::from_msg(msg.to_string()).into_js_error());
        }

        let IntrospectionResult {
            datamodels,
            views,
            warnings,
            is_empty,
        } = self.connector.introspect(&ctx).await?;

        if is_empty {
            Err(ConnectorError::into_introspection_result_empty_error().into_js_error())
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
    }

    /// Introspects a SQL query and returns types information.
    /// Note: this will fail on SQLite, as it requires Wasm-compatible sqlx implementation.
    #[wasm_bindgen(js_name = "introspectSql")]
    pub async fn introspect_sql(
        &mut self,
        params: IntrospectSqlParams,
    ) -> Result<IntrospectSqlResult, wasm_bindgen::JsError> {
        let res = commands::introspect_sql(params, &mut self.connector).await?;

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
    }

    /// Mark a migration from the migrations folder as applied, without actually applying it.
    #[wasm_bindgen(js_name = "markMigrationApplied")]
    pub async fn mark_migration_applied(
        &mut self,
        input: MarkMigrationAppliedInput,
    ) -> Result<MarkMigrationAppliedOutput, wasm_bindgen::JsError> {
        let span = tracing::info_span!("MarkMigrationApplied", migration_name = input.migration_name.as_str());
        let result = commands::mark_migration_applied(input, &mut self.connector)
            .instrument(span)
            .await?;
        Ok(result)
    }

    /// Mark a migration as rolled back.
    #[wasm_bindgen(js_name = "markMigrationRolledBack")]
    pub async fn mark_migration_rolled_back(
        &mut self,
        input: MarkMigrationRolledBackInput,
    ) -> Result<MarkMigrationRolledBackOutput, wasm_bindgen::JsError> {
        let span = tracing::info_span!(
            "MarkMigrationRolledBack",
            migration_name = input.migration_name.as_str()
        );
        let result = commands::mark_migration_rolled_back(input, &mut self.connector)
            .instrument(span)
            .await?;
        Ok(result)
    }

    /// Reset a database to an empty state (no data, no schema).
    #[wasm_bindgen]
    pub async fn reset(&mut self) -> Result<(), wasm_bindgen::JsError> {
        tracing::debug!("Resetting the database.");
        let namespaces = self.namespaces();

        let result = SchemaConnector::reset(&mut self.connector, false, namespaces)
            .instrument(tracing::info_span!("Reset"))
            .await?;
        Ok(result)
    }

    /// The command behind `prisma db push`.
    #[wasm_bindgen(js_name = "schemaPush")]
    pub async fn schema_push(&mut self, input: SchemaPushInput) -> Result<SchemaPushOutput, wasm_bindgen::JsError> {
        let result = commands::schema_push(input, &mut self.connector)
            .instrument(tracing::info_span!("SchemaPush"))
            .await?;
        Ok(result)
    }
}
