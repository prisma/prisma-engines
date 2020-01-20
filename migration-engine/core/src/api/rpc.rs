use super::{GenericApi, MigrationApi};
use crate::{commands::*, CoreResult};
use datamodel::configuration::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME};
use futures::{FutureExt, TryFutureExt};
use jsonrpc_core::types::error::Error as JsonRpcError;
use jsonrpc_core::{IoHandler, Params};
use jsonrpc_stdio_server::ServerBuilder;
use sql_migration_connector::SqlMigrationConnector;
use std::{io, sync::Arc};
use thiserror::Error;

pub struct RpcApi {
    io_handler: jsonrpc_core::IoHandler<()>,
    executor: Arc<dyn GenericApi>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RpcCommand {
    InferMigrationSteps,
    ListMigrations,
    MigrationProgress,
    ApplyMigration,
    UnapplyMigration,
    Reset,
    CalculateDatamodel,
    CalculateDatabaseSteps,
}

impl RpcCommand {
    fn name(&self) -> &'static str {
        match self {
            RpcCommand::InferMigrationSteps => "inferMigrationSteps",
            RpcCommand::ListMigrations => "listMigrations",
            RpcCommand::MigrationProgress => "migrationProgress",
            RpcCommand::ApplyMigration => "applyMigration",
            RpcCommand::UnapplyMigration => "unapplyMigration",
            RpcCommand::Reset => "reset",
            RpcCommand::CalculateDatamodel => "calculateDatamodel",
            RpcCommand::CalculateDatabaseSteps => "calculateDatabaseSteps",
        }
    }
}

static AVAILABLE_COMMANDS: &[RpcCommand] = &[
    RpcCommand::ApplyMigration,
    RpcCommand::InferMigrationSteps,
    RpcCommand::ListMigrations,
    RpcCommand::MigrationProgress,
    RpcCommand::UnapplyMigration,
    RpcCommand::Reset,
    RpcCommand::CalculateDatamodel,
    RpcCommand::CalculateDatabaseSteps,
];

impl RpcApi {
    pub async fn new(datamodel: &str) -> CoreResult<Self> {
        let config = datamodel::parse_configuration(datamodel)?;

        let source = config.datasources.first().ok_or(CommandError::DataModelErrors {
            errors: vec!["There is no datasource in the configuration.".to_string()],
        })?;

        let connector = match source.connector_type() {
            provider if [MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&provider) => {
                SqlMigrationConnector::new(&source.url().value, provider).await?
            }
            x => unimplemented!("Connector {} is not supported yet", x),
        };

        let mut rpc_api = Self {
            io_handler: IoHandler::default(),
            executor: Arc::new(MigrationApi::new(connector).await?),
        };

        for cmd in AVAILABLE_COMMANDS {
            rpc_api.add_command_handler(*cmd);
        }

        Ok(rpc_api)
    }

    /// Block the thread and handle IO in async until EOF.
    pub async fn start_server(self) {
        ServerBuilder::new(self.io_handler).build()
    }

    /// Handle one request
    pub fn handle(&self) -> CoreResult<String> {
        let mut json_is_complete = false;
        let mut input = String::new();

        while !json_is_complete {
            io::stdin().read_line(&mut input)?;
            json_is_complete = serde_json::from_str::<serde_json::Value>(&input).is_ok();
        }

        let result = self
            .io_handler
            .handle_request_sync(&input)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Reading from stdin failed."))?;

        Ok(result)
    }

    fn add_command_handler(&mut self, cmd: RpcCommand) {
        let executor = Arc::clone(&self.executor);

        self.io_handler.add_method(cmd.name(), move |params: Params| {
            let cmd = cmd.clone();
            let executor = Arc::clone(&executor);
            let fut = async move { Self::create_handler(&executor, cmd, &params).await };

            fut.boxed().compat()
        });
    }

    async fn create_handler(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: &Params,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let result: Result<serde_json::Value, RunCommandError> = Self::run_command(&executor, cmd, params).await;

        match result {
            Ok(result) => Ok(result),
            Err(RunCommandError::JsonRpcError(err)) => Err(err),
            Err(RunCommandError::CrateError(err)) => Err(executor.render_jsonrpc_error(err)),
        }
    }

    async fn run_command(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: &Params,
    ) -> Result<serde_json::Value, RunCommandError> {
        tracing::debug!(?cmd, "running the command");
        match cmd {
            RpcCommand::InferMigrationSteps => {
                let input: InferMigrationStepsInput = params.clone().parse()?;
                render(executor.infer_migration_steps(&input).await?)
            }
            RpcCommand::ListMigrations => render(executor.list_migrations(&serde_json::Value::Null).await?),
            RpcCommand::MigrationProgress => {
                let input: MigrationProgressInput = params.clone().parse()?;
                render(executor.migration_progress(&input).await?)
            }
            RpcCommand::ApplyMigration => {
                let input: ApplyMigrationInput = params.clone().parse()?;
                let result = executor.apply_migration(&input).await?;
                tracing::debug!("command result: {:?}", result);
                render(result)
            }
            RpcCommand::UnapplyMigration => {
                let input: UnapplyMigrationInput = params.clone().parse()?;
                render(executor.unapply_migration(&input).await?)
            }
            RpcCommand::Reset => render(executor.reset(&serde_json::Value::Null).await?),
            RpcCommand::CalculateDatamodel => {
                let input: CalculateDatamodelInput = params.clone().parse()?;
                render(executor.calculate_datamodel(&input).await?)
            }
            RpcCommand::CalculateDatabaseSteps => {
                let input: CalculateDatabaseStepsInput = params.clone().parse()?;
                render(executor.calculate_database_steps(&input).await?)
            }
        }
    }
}

fn render(result: impl serde::Serialize) -> Result<serde_json::Value, RunCommandError> {
    Ok(serde_json::to_value(result).expect("Rendering of RPC response failed"))
}

#[derive(Debug, Error)]
enum RunCommandError {
    #[error("{0}")]
    JsonRpcError(JsonRpcError),
    #[error("{0}")]
    CrateError(crate::Error),
}

impl From<JsonRpcError> for RunCommandError {
    fn from(e: JsonRpcError) -> Self {
        RunCommandError::JsonRpcError(e)
    }
}

impl From<crate::Error> for RunCommandError {
    fn from(e: crate::Error) -> Self {
        RunCommandError::CrateError(e)
    }
}
