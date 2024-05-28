use crate::{json_rpc::method_names::*, CoreError, CoreResult, GenericApi};
use jsonrpc_core::{types::error::Error as JsonRpcError, IoHandler, Params};
use psl::SourceFile;
use std::sync::Arc;

/// Initialize a JSON-RPC ready schema engine API.
pub fn rpc_api(
    initial_datamodels: Option<Vec<(String, String)>>,
    host: Arc<dyn schema_connector::ConnectorHost>,
) -> IoHandler {
    let mut io_handler = IoHandler::default();
    let initial_datamodels = initial_datamodels.map(|schemas| {
        schemas
            .into_iter()
            .map(|(name, schema)| (name, SourceFile::from(schema)))
            .collect()
    });

    let api = Arc::new(crate::state::EngineState::new(initial_datamodels, Some(host)));

    for cmd in METHOD_NAMES {
        let api = api.clone();
        io_handler.add_method(cmd, move |params: Params| {
            Box::pin(run_command(api.clone(), cmd, params))
        });
    }

    io_handler
}

#[allow(clippy::redundant_allocation)]
async fn run_command(
    executor: Arc<dyn GenericApi>,
    cmd: &str,
    params: Params,
) -> Result<serde_json::Value, JsonRpcError> {
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
        LIST_MIGRATION_DIRECTORIES => render(executor.list_migration_directories(params.parse()?).await),
        MARK_MIGRATION_APPLIED => render(executor.mark_migration_applied(params.parse()?).await),
        MARK_MIGRATION_ROLLED_BACK => render(executor.mark_migration_rolled_back(params.parse()?).await),
        // TODO(MultiSchema): we probably need to grab the namespaces from the params
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
