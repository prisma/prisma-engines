use crate::{CoreError, CoreResult, GenericApi};
use jsonrpc_core::{types::error::Error as JsonRpcError, IoHandler, Params};
use std::sync::Arc;

const APPLY_MIGRATIONS: &str = "applyMigrations";
const CREATE_MIGRATION: &str = "createMigration";
const DEBUG_PANIC: &str = "debugPanic";
const DEV_DIAGNOSTIC: &str = "devDiagnostic";
const DIAGNOSE_MIGRATION_HISTORY: &str = "diagnoseMigrationHistory";
const EVALUATE_DATA_LOSS: &str = "evaluateDataLoss";
const GET_DATABASE_VERSION: &str = "getDatabaseVersion";
const LIST_MIGRATION_DIRECTORIES: &str = "listMigrationDirectories";
const MARK_MIGRATION_APPLIED: &str = "markMigrationApplied";
const MARK_MIGRATION_ROLLED_BACK: &str = "markMigrationRolledBack";
const PLAN_MIGRATION: &str = "planMigration";
const RESET: &str = "reset";
const SCHEMA_PUSH: &str = "schemaPush";

const AVAILABLE_COMMANDS: &[&str] = &[
    APPLY_MIGRATIONS,
    CREATE_MIGRATION,
    DEBUG_PANIC,
    DEV_DIAGNOSTIC,
    DIAGNOSE_MIGRATION_HISTORY,
    EVALUATE_DATA_LOSS,
    GET_DATABASE_VERSION,
    LIST_MIGRATION_DIRECTORIES,
    MARK_MIGRATION_APPLIED,
    MARK_MIGRATION_ROLLED_BACK,
    PLAN_MIGRATION,
    RESET,
    SCHEMA_PUSH,
];

/// Initialize a JSON-RPC ready migration engine API. This entails starting
/// a database connection.
pub async fn rpc_api(datamodel: &str) -> CoreResult<IoHandler> {
    let mut io_handler = IoHandler::default();
    let executor = Arc::new(crate::migration_api(datamodel).await?);

    for cmd in AVAILABLE_COMMANDS {
        let executor = executor.clone();
        io_handler.add_method(cmd, move |params: Params| {
            Box::pin(run_command(executor.clone(), cmd, params))
        });
    }

    Ok(io_handler)
}

async fn run_command(
    executor: Arc<Box<dyn GenericApi>>,
    cmd: &str,
    params: Params,
) -> Result<serde_json::Value, JsonRpcError> {
    tracing::debug!(?cmd, "running the command");
    match cmd {
        APPLY_MIGRATIONS => render(executor.apply_migrations(&params.parse()?).await),
        CREATE_MIGRATION => render(executor.create_migration(&params.parse()?).await),
        DEV_DIAGNOSTIC => render(executor.dev_diagnostic(&params.parse()?).await),
        DEBUG_PANIC => render(executor.debug_panic().await),
        DIAGNOSE_MIGRATION_HISTORY => render(executor.diagnose_migration_history(&params.parse()?).await),
        EVALUATE_DATA_LOSS => render(executor.evaluate_data_loss(&params.parse()?).await),
        GET_DATABASE_VERSION => render(executor.version().await),
        LIST_MIGRATION_DIRECTORIES => render(executor.list_migration_directories(&params.parse()?).await),
        MARK_MIGRATION_APPLIED => render(executor.mark_migration_applied(&params.parse()?).await),
        MARK_MIGRATION_ROLLED_BACK => render(executor.mark_migration_rolled_back(&params.parse()?).await),
        PLAN_MIGRATION => render(executor.plan_migration().await),
        RESET => render(executor.reset().await),
        SCHEMA_PUSH => render(executor.schema_push(&params.parse()?).await),
        other => unreachable!("Unknown command {}", other),
    }
}

fn render(result: CoreResult<impl serde::Serialize>) -> jsonrpc_core::Result<jsonrpc_core::Value> {
    result
        .map(|response| serde_json::to_value(response).expect("Rendering of RPC response failed"))
        .map_err(render_jsonrpc_error)
}

fn render_jsonrpc_error(crate_error: CoreError) -> JsonRpcError {
    serde_json::to_value(&crate_error.to_user_facing())
        .map(|data| JsonRpcError {
            // We separate the JSON-RPC error code (defined by the JSON-RPC spec) from the
            // prisma error code, which is located in `data`.
            code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
            message: "An error happened. Check the data field for details.".to_string(),
            data: Some(data),
        })
        .unwrap()
}
