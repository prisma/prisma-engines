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
    /// When there was a bad datamodel as part of the input.
    #[error("{0}")]
    ReceivedBadDatamodel(String),

    /// When a datamodel from a generated AST is wrong. This is basically an internal error.
    #[error("The migration produced an invalid schema ({0:?})")]
    ProducedBadDatamodel(datamodel::error::ErrorCollection),

    #[error("Initialization error. (error: {0})")]
    InitializationError(anyhow::Error),

    #[error("Connector error. (error: {0})")]
    ConnectorError(ConnectorError),

    #[error("Generic error. (error: {0})")]
    Generic(anyhow::Error),

    #[error("Error in command input. (error: {0})")]
    Input(anyhow::Error),
}

impl From<migration_connector::ConnectorError> for CommandError {
    fn from(error: migration_connector::ConnectorError) -> CommandError {
        CommandError::ConnectorError(error)
    }
}

impl From<CalculatorError> for CommandError {
    fn from(error: CalculatorError) -> Self {
        CommandError::Generic(error.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_error_produced_bad_datamodel_is_intelligible() {
        let bad_dml = r#"
            model Test {
                id Float @id
                post Post[]
            }
        "#;

        let err = datamodel::parse_datamodel(bad_dml)
            .map_err(CommandError::ProducedBadDatamodel)
            .unwrap_err();

        assert_eq!(
            err.to_string(),
            "The migration produced an invalid schema (ErrorCollection { errors: [TypeNotFoundError { type_name: \"Post\", span: Span { start: 76, end: 82 } }] })"
        )
    }
}
