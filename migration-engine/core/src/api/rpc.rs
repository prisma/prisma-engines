use super::error_rendering::render_jsonrpc_error;
use crate::{CoreError, CoreResult, GenericApi};
use enumflags2::BitFlags;
use futures::{FutureExt, TryFutureExt};
use jsonrpc_core::{types::error::Error as JsonRpcError, IoHandler, Params};
use migration_connector::MigrationFeature;
use std::sync::Arc;

pub struct RpcApi {
    io_handler: jsonrpc_core::IoHandler<()>,
    executor: Arc<dyn GenericApi>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RpcCommand {
    ApplyMigrations,
    ApplyScript,
    CreateMigration,
    DebugPanic,
    DevDiagnostic,
    DiagnoseMigrationHistory,
    EvaluateDataLoss,
    GetDatabaseVersion,
    ListMigrationDirectories,
    MarkMigrationApplied,
    MarkMigrationRolledBack,
    PlanMigration,
    Reset,
    SchemaPush,
}

impl RpcCommand {
    fn name(&self) -> &'static str {
        match self {
            RpcCommand::ApplyMigrations => "applyMigrations",
            RpcCommand::ApplyScript => "applyScript",
            RpcCommand::CreateMigration => "createMigration",
            RpcCommand::DebugPanic => "debugPanic",
            RpcCommand::DevDiagnostic => "devDiagnostic",
            RpcCommand::DiagnoseMigrationHistory => "diagnoseMigrationHistory",
            RpcCommand::EvaluateDataLoss => "evaluateDataLoss",
            RpcCommand::GetDatabaseVersion => "getDatabaseVersion",
            RpcCommand::ListMigrationDirectories => "listMigrationDirectories",
            RpcCommand::MarkMigrationApplied => "markMigrationApplied",
            RpcCommand::MarkMigrationRolledBack => "markMigrationRolledBack",
            RpcCommand::PlanMigration => "planMigration",
            RpcCommand::Reset => "reset",
            RpcCommand::SchemaPush => "schemaPush",
        }
    }
}

const AVAILABLE_COMMANDS: &[RpcCommand] = &[
    RpcCommand::ApplyMigrations,
    RpcCommand::ApplyScript,
    RpcCommand::CreateMigration,
    RpcCommand::DebugPanic,
    RpcCommand::DevDiagnostic,
    RpcCommand::DiagnoseMigrationHistory,
    RpcCommand::EvaluateDataLoss,
    RpcCommand::GetDatabaseVersion,
    RpcCommand::ListMigrationDirectories,
    RpcCommand::MarkMigrationApplied,
    RpcCommand::MarkMigrationRolledBack,
    RpcCommand::PlanMigration,
    RpcCommand::Reset,
    RpcCommand::SchemaPush,
];

impl RpcApi {
    pub async fn new(datamodel: &str, enabled_preview_features: BitFlags<MigrationFeature>) -> CoreResult<Self> {
        let mut rpc_api = Self {
            io_handler: IoHandler::default(),
            executor: crate::migration_api(datamodel, enabled_preview_features).await?,
        };

        for cmd in AVAILABLE_COMMANDS {
            rpc_api.add_command_handler(*cmd);
        }

        Ok(rpc_api)
    }

    pub fn io_handler(&self) -> &IoHandler {
        &self.io_handler
    }

    fn add_command_handler(&mut self, cmd: RpcCommand) {
        let executor = Arc::clone(&self.executor);

        self.io_handler.add_method(cmd.name(), move |params: Params| {
            let executor = Arc::clone(&executor);
            Self::create_handler(executor, cmd, params).boxed().compat()
        });
    }

    async fn create_handler(
        executor: Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: Params,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let result: Result<serde_json::Value, RunCommandError> = Self::run_command(&executor, cmd, params).await;

        match result {
            Ok(result) => Ok(result),
            Err(RunCommandError::JsonRpcError(err)) => Err(err),
            Err(RunCommandError::CoreError(err)) => Err(render_jsonrpc_error(err)),
        }
    }

    async fn run_command(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: Params,
    ) -> Result<serde_json::Value, RunCommandError> {
        tracing::debug!(?cmd, "running the command");
        Ok(match cmd {
            RpcCommand::ApplyScript => render(executor.apply_script(&params.parse()?).await?),
            RpcCommand::ApplyMigrations => render(executor.apply_migrations(&params.parse()?).await?),
            RpcCommand::CreateMigration => render(executor.create_migration(&params.parse()?).await?),
            RpcCommand::DevDiagnostic => render(executor.dev_diagnostic(&params.parse()?).await?),
            RpcCommand::DebugPanic => render(executor.debug_panic(&()).await?),
            RpcCommand::DiagnoseMigrationHistory => {
                render(executor.diagnose_migration_history(&params.parse()?).await?)
            }
            RpcCommand::EvaluateDataLoss => render(executor.evaluate_data_loss(&params.parse()?).await?),
            RpcCommand::GetDatabaseVersion => render(executor.version(&serde_json::Value::Null).await?),
            RpcCommand::ListMigrationDirectories => {
                render(executor.list_migration_directories(&params.parse()?).await?)
            }
            RpcCommand::MarkMigrationApplied => render(executor.mark_migration_applied(&params.parse()?).await?),
            RpcCommand::MarkMigrationRolledBack => render(executor.mark_migration_rolled_back(&params.parse()?).await?),
            RpcCommand::PlanMigration => render(executor.plan_migration(&params.parse()?).await?),
            RpcCommand::Reset => render(executor.reset(&()).await?),
            RpcCommand::SchemaPush => render(executor.schema_push(&params.parse()?).await?),
        })
    }
}

fn render(result: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(result).expect("Rendering of RPC response failed")
}

enum RunCommandError {
    JsonRpcError(JsonRpcError),
    CoreError(CoreError),
}

impl From<JsonRpcError> for RunCommandError {
    fn from(e: JsonRpcError) -> Self {
        RunCommandError::JsonRpcError(e)
    }
}

impl From<CoreError> for RunCommandError {
    fn from(e: CoreError) -> Self {
        RunCommandError::CoreError(e)
    }
}
