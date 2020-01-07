use crate::migration::datamodel_calculator::CalculatorError;
use crate::migration_engine::MigrationEngine;
use migration_connector::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

#[async_trait::async_trait]
pub trait MigrationCommand {
    type Input: DeserializeOwned;
    type Output: Serialize + 'static;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static;
}

pub type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Errors in datamodel. (errors: {:?})", errors)]
    DataModelErrors { errors: Vec<String> },

    #[error("Initialization error. (error: {})", error)]
    InitializationError { error: String },

    #[error("Connector error. (error: {0})")]
    ConnectorError(ConnectorError),

    #[error("Generic error. (error: {})", error)]
    Generic { error: String },

    #[error("Error in command input. (error: {})", error)]
    Input { error: String },
}

impl From<datamodel::error::ErrorCollection> for CommandError {
    fn from(errors: datamodel::error::ErrorCollection) -> CommandError {
        let errors_str = errors
            .errors
            .into_iter()
            .map(|e| {
                // let mut msg: Vec<u8> = Vec::new();
                // e.pretty_print(&mut msg, "datamodel", "bla").unwrap();
                // std::str::from_utf8(&msg).unwrap().to_string()
                format!("{}", e)
            })
            .collect();
        CommandError::DataModelErrors { errors: errors_str }
    }
}

impl From<migration_connector::ConnectorError> for CommandError {
    fn from(error: migration_connector::ConnectorError) -> CommandError {
        CommandError::ConnectorError(error)
    }
}

impl From<CalculatorError> for CommandError {
    fn from(error: CalculatorError) -> Self {
        CommandError::Generic {
            error: format!("{}", error),
        }
    }
}
