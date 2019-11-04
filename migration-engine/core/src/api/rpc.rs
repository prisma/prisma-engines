use super::{GenericApi, MigrationApi};
use crate::commands::*;
use datamodel::configuration::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME};
use failure::Fail;
use futures::{
    future::{err, lazy, ok, poll_fn},
    Future,
};
use jsonrpc_core::types::error::Error as JsonRpcError;
use jsonrpc_core::IoHandler;
use jsonrpc_core::*;
use jsonrpc_stdio_server::ServerBuilder;
use sql_migration_connector::SqlMigrationConnector;
use std::{io, sync::Arc};
use tokio_threadpool::blocking;

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
    pub fn new_async(datamodel: &str) -> crate::Result<Self> {
        let mut rpc_api = Self::new(datamodel)?;

        for cmd in AVAILABLE_COMMANDS {
            rpc_api.add_async_command_handler(*cmd);
        }

        Ok(rpc_api)
    }

    pub fn new_sync(datamodel: &str) -> crate::Result<Self> {
        let mut rpc_api = Self::new(datamodel)?;

        for cmd in AVAILABLE_COMMANDS {
            rpc_api.add_sync_command_handler(*cmd);
        }

        Ok(rpc_api)
    }

    /// Block the thread and handle IO in async until EOF.
    pub fn start_server(self) {
        ServerBuilder::new(self.io_handler).build()
    }

    /// Handle one request
    pub fn handle(&self) -> crate::Result<String> {
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

    fn new(datamodel: &str) -> crate::Result<RpcApi> {
        let config = datamodel::parse_configuration(datamodel)?;

        let source = config.datasources.first().ok_or(CommandError::DataModelErrors {
            code: 1000,
            errors: vec!["There is no datasource in the configuration.".to_string()],
        })?;

        let connector = match source.connector_type() {
            scheme if [MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&scheme) => {
                SqlMigrationConnector::new(source.as_ref())?
            }
            x => unimplemented!("Connector {} is not supported yet", x),
        };

        Ok(Self {
            io_handler: IoHandler::default(),
            executor: Arc::new(MigrationApi::new(connector)?),
        })
    }

    fn add_sync_command_handler(&mut self, cmd: RpcCommand) {
        let executor = Arc::clone(&self.executor);

        self.io_handler.add_method(cmd.name(), move |params: Params| {
            Self::create_sync_handler(&executor, cmd, &params)
        });
    }

    fn add_async_command_handler(&mut self, cmd: RpcCommand) {
        let executor = Arc::clone(&self.executor);

        self.io_handler.add_method(cmd.name(), move |params: Params| {
            Self::create_async_handler(&executor, cmd, params)
        });
    }

    fn create_sync_handler(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: &Params,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        use std::result::Result;

        let result: Result<Result<serde_json::Value, RunCommandError>, _> = {
            let executor = AssertUnwindSafe(executor);
            catch_unwind(|| Self::run_command(&executor, cmd, params))
        };

        match result {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(RunCommandError::JsonRpcError(err))) => Err(err),
            Ok(Err(RunCommandError::CrateError(err))) => Err(executor.render_jsonrpc_error(err)),
            Err(panic) => Err(executor.render_panic(panic)),
        }
    }

    fn create_async_handler(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: Params,
    ) -> impl Future<Item = serde_json::Value, Error = JsonRpcError> {
        let executor = Arc::clone(executor);

        lazy(move || poll_fn(move || blocking(|| Self::create_sync_handler(&executor, cmd, &params)))).then(|res| {
            match res {
                // dumdidum futures 0.1 we love <3
                Ok(Ok(val)) => ok(val),
                Ok(Err(val)) => err(val),
                Err(val) => {
                    let e = crate::error::Error::from(val);
                    err(super::error_rendering::render_jsonrpc_error(e))
                }
            }
        })
    }

    fn run_command(
        executor: &Arc<dyn GenericApi>,
        cmd: RpcCommand,
        params: &Params,
    ) -> std::result::Result<serde_json::Value, RunCommandError> {
        use log::debug;
        debug!("running the command");
        match cmd {
            RpcCommand::InferMigrationSteps => {
                let input: InferMigrationStepsInput = params.clone().parse()?;
                render(executor.infer_migration_steps(&input)?)
            }
            RpcCommand::ListMigrations => render(executor.list_migrations(&serde_json::Value::Null)?),
            RpcCommand::MigrationProgress => {
                let input: MigrationProgressInput = params.clone().parse()?;
                render(executor.migration_progress(&input)?)
            }
            RpcCommand::ApplyMigration => {
                let input: ApplyMigrationInput = params.clone().parse()?;
                let result = executor.apply_migration(&input)?;
                debug!("command result: {:?}", result);
                render(result)
            }
            RpcCommand::UnapplyMigration => {
                let input: UnapplyMigrationInput = params.clone().parse()?;
                render(executor.unapply_migration(&input)?)
            }
            RpcCommand::Reset => render(executor.reset(&serde_json::Value::Null)?),
            RpcCommand::CalculateDatamodel => {
                let input: CalculateDatamodelInput = params.clone().parse()?;
                render(executor.calculate_datamodel(&input)?)
            }
            RpcCommand::CalculateDatabaseSteps => {
                let input: CalculateDatabaseStepsInput = params.clone().parse()?;
                render(executor.calculate_database_steps(&input)?)
            }
        }
    }
}

fn render(result: impl serde::Serialize) -> std::result::Result<serde_json::Value, RunCommandError> {
    Ok(serde_json::to_value(result).expect("Rendering of RPC response failed"))
}

#[derive(Debug, Fail)]
enum RunCommandError {
    #[fail(display = "{}", _0)]
    JsonRpcError(JsonRpcError),
    #[fail(display = "{}", _0)]
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
