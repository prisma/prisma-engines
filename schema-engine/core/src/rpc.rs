use crate::{CoreError, CoreResult, GenericApi};
use json_rpc::method_names::*;
use jsonrpc_core::{types::error::Error as JsonRpcError, IoHandler, Params};
use psl::SourceFile;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Stateful JSON-RPC API wrapper.
pub struct RpcApi {
    io_handler: IoHandler,
    api: Arc<RwLock<dyn GenericApi>>,
}

impl RpcApi {
    /// Initializes a JSON-RPC ready schema engine API.
    pub fn new(
        initial_datamodels: Option<Vec<(String, String)>>,
        host: Arc<dyn schema_connector::ConnectorHost>,
    ) -> Self {
        let mut io_handler = IoHandler::default();
        let initial_datamodels = initial_datamodels.map(|schemas| {
            schemas
                .into_iter()
                .map(|(name, schema)| (name, SourceFile::from(schema)))
                .collect()
        });

        let api = Arc::new(RwLock::new(crate::state::EngineState::new(
            initial_datamodels,
            Some(host),
        )));

        for cmd in METHOD_NAMES {
            let api = api.clone();
            io_handler.add_method(cmd, move |params: Params| {
                Box::pin(run_command(api.clone(), cmd, params))
            });
        }

        Self { io_handler, api }
    }

    /// Returns the underlying JSON-RPC handler.
    pub fn io_handler(&self) -> &IoHandler {
        &self.io_handler
    }

    /// Disposes the database connectors and drops the JSON-RPC handler.
    /// It is not strictly necessary to call this method when dealing with most
    /// well-behaved databases, but it ensures that the connections are always
    /// closed politely and gracefully, which is required, e.g., for PGlite.
    /// If not called, there will be no resource leaks or correctness issues
    /// on our side, but the database might not be notified about the shutdown.
    pub async fn dispose(self) -> CoreResult<()> {
        self.api.write().await.dispose().await
    }
}

async fn run_command(
    executor: Arc<RwLock<dyn GenericApi>>,
    cmd: &str,
    params: Params,
) -> Result<serde_json::Value, JsonRpcError> {
    let executor = executor.read_owned().await;
    match cmd {
        APPLY_MIGRATIONS => render(executor.apply_migrations(params.parse()?).await),
        CREATE_DATABASE => render(executor.create_database(params.parse()?).await),
        CREATE_MIGRATION => render(executor.create_migration(params.parse()?).await),
        DB_EXECUTE => render(executor.db_execute(params.parse()?).await),
        DEV_DIAGNOSTIC => render(executor.dev_diagnostic(params.parse()?).await),
        DIFF => render(executor.diff(params.parse()?).await),
        DEBUG_PANIC => render(executor.debug_panic().await),
        DIAGNOSE_MIGRATION_HISTORY => render(executor.diagnose_migration_history(params.parse()?).await),
        ENSURE_CONNECTION_VALIDITY => render(executor.ensure_connection_validity(params.parse()?).await),
        EVALUATE_DATA_LOSS => render(executor.evaluate_data_loss(params.parse()?).await),
        GET_DATABASE_VERSION => render(executor.version(params.parse()?).await),
        INTROSPECT => render(executor.introspect(params.parse()?).await),
        INTROSPECT_SQL => render(executor.introspect_sql(params.parse()?).await),
        MARK_MIGRATION_APPLIED => render(executor.mark_migration_applied(params.parse()?).await),
        MARK_MIGRATION_ROLLED_BACK => render(executor.mark_migration_rolled_back(params.parse()?).await),
        RESET => render(executor.reset().await),
        SCHEMA_PUSH => render(executor.schema_push(params.parse()?).await),
        other => unreachable!("Unknown command {}", other),
    }
}

fn render(result: CoreResult<impl serde::Serialize>) -> jsonrpc_core::Result<jsonrpc_core::Value> {
    result
        .map(|response| serde_json::to_value(response).expect("Rendering of RPC response failed"))
        .map_err(render_jsonrpc_error)
}

fn render_jsonrpc_error(crate_error: CoreError) -> JsonRpcError {
    serde_json::to_value(crate_error.to_user_facing())
        .map(|data| JsonRpcError {
            // We separate the JSON-RPC error code (defined by the JSON-RPC spec) from the
            // prisma error code, which is located in `data`.
            code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
            message: "An error happened. Check the data field for details.".to_string(),
            data: Some(data),
        })
        .unwrap()
}
