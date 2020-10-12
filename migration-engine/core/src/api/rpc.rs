use crate::{CoreError, CoreResult, GenericApi};
use futures::{FutureExt, TryFutureExt};
use jsonrpc_core::{types::error::Error as JsonRpcError, IoHandler, Params};
use std::sync::Arc;

pub struct RpcApi {
    io_handler: jsonrpc_core::IoHandler<()>,
    executor: Arc<dyn GenericApi>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RpcCommand {
    GetDatabaseVersion,
    ApplyMigrations,
    CreateMigration,
    DebugPanic,
    DiagnoseMigrationHistory,
    EvaluateDataLoss,
    InferMigrationSteps,
    Initialize,
    ListMigrations,
    MigrationProgress,
    PlanMigration,
    ApplyMigration,
    UnapplyMigration,
    Reset,
    SchemaPush,
    CalculateDatamodel,
    CalculateDatabaseSteps,
}

impl RpcCommand {
    fn name(&self) -> &'static str {
        match self {
            RpcCommand::GetDatabaseVersion => "getDatabaseVersion",
            RpcCommand::ApplyMigrations => "applyMigrations",
            RpcCommand::CreateMigration => "createMigration",
            RpcCommand::DebugPanic => "debugPanic",
            RpcCommand::DiagnoseMigrationHistory => "diagnoseMigrationHistory",
            RpcCommand::EvaluateDataLoss => "evaluateDataLoss",
            RpcCommand::InferMigrationSteps => "inferMigrationSteps",
            RpcCommand::ListMigrations => "listMigrations",
            RpcCommand::MigrationProgress => "migrationProgress",
            RpcCommand::ApplyMigration => "applyMigration",
            RpcCommand::UnapplyMigration => "unapplyMigration",
            RpcCommand::Initialize => "initialize",
            RpcCommand::PlanMigration => "planMigration",
            RpcCommand::Reset => "reset",
            RpcCommand::SchemaPush => "schemaPush",
            RpcCommand::CalculateDatamodel => "calculateDatamodel",
            RpcCommand::CalculateDatabaseSteps => "calculateDatabaseSteps",
        }
    }
}

const AVAILABLE_COMMANDS: &[RpcCommand] = &[
    RpcCommand::GetDatabaseVersion,
    RpcCommand::ApplyMigration,
    RpcCommand::ApplyMigrations,
    RpcCommand::CreateMigration,
    RpcCommand::DiagnoseMigrationHistory,
    RpcCommand::EvaluateDataLoss,
    RpcCommand::DebugPanic,
    RpcCommand::InferMigrationSteps,
    RpcCommand::Initialize,
    RpcCommand::ListMigrations,
    RpcCommand::MigrationProgress,
    RpcCommand::PlanMigration,
    RpcCommand::UnapplyMigration,
    RpcCommand::Reset,
    RpcCommand::SchemaPush,
    RpcCommand::CalculateDatamodel,
    RpcCommand::CalculateDatabaseSteps,
];

impl RpcApi {
    pub async fn new(datamodel: &str) -> CoreResult<Self> {
        let mut rpc_api = Self {
            io_handler: IoHandler::default(),
            executor: crate::migration_api(datamodel).await?,
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
            Err(RunCommandError::CoreError(err)) => Err(executor.render_jsonrpc_error(err)),
        }
    }

    async fn run_command(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: Params,
    ) -> Result<serde_json::Value, RunCommandError> {
        tracing::debug!(?cmd, "running the command");
        match cmd {
            RpcCommand::ApplyMigrations => render(executor.apply_migrations(&params.parse()?).await?),
            RpcCommand::CreateMigration => render(executor.create_migration(&params.parse()?).await?),
            RpcCommand::DebugPanic => render(executor.debug_panic(&()).await?),
            RpcCommand::ApplyMigration => render(executor.apply_migration(&params.parse()?).await?),
            RpcCommand::CalculateDatabaseSteps => render(executor.calculate_database_steps(&params.parse()?).await?),
            RpcCommand::CalculateDatamodel => render(executor.calculate_datamodel(&params.parse()?).await?),
            RpcCommand::DiagnoseMigrationHistory => {
                render(executor.diagnose_migration_history(&params.parse()?).await?)
            }
            RpcCommand::EvaluateDataLoss => render(executor.evaluate_data_loss(&params.parse()?).await?),
            RpcCommand::GetDatabaseVersion => render(executor.version(&serde_json::Value::Null).await?),
            RpcCommand::InferMigrationSteps => render(executor.infer_migration_steps(&params.parse()?).await?),
            RpcCommand::Initialize => render(executor.initialize(&params.parse()?).await?),
            RpcCommand::ListMigrations => render(executor.list_migrations(&serde_json::Value::Null).await?),
            RpcCommand::MigrationProgress => render(executor.migration_progress(&params.parse()?).await?),
            RpcCommand::PlanMigration => render(executor.plan_migration(&params.parse()?).await?),
            RpcCommand::Reset => render(executor.reset(&()).await?),
            RpcCommand::SchemaPush => render(executor.schema_push(&params.parse()?).await?),
            RpcCommand::UnapplyMigration => render(executor.unapply_migration(&params.parse()?).await?),
        }
    }
}

fn render(result: impl serde::Serialize) -> Result<serde_json::Value, RunCommandError> {
    Ok(serde_json::to_value(result).expect("Rendering of RPC response failed"))
}

#[derive(Debug)]
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
