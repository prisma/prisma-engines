use crate::{migration::datamodel_calculator::CalculatorError, migration_engine::MigrationEngine};
use migration_connector::*;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// The implementation of an RPC command exposed by the migration engine.
#[async_trait::async_trait]
pub trait MigrationCommand {
    /// The input parameters to the command.
    type Input: DeserializeOwned;
    /// The response shape of the command.
    type Output: Serialize + 'static;

    /// Handle the input, producing the response or an error.
    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static;
}

/// The result type for migration engine commands.
pub type CommandResult<T> = Result<T, CommandError>;

/// The top-level error type for migration engine commands.
#[derive(Debug, Error)]
pub enum CommandError {
    /// When there was a bad datamodel as part of the input.
    #[error("{0}")]
    ReceivedBadDatamodel(String),

    /// When a datamodel from a generated AST is wrong. This is basically an internal error.
    #[error("The migration produced an invalid schema.\n{}", render_datamodel_error(.0, None))]
    ProducedBadDatamodel(datamodel::error::ErrorCollection),

    /// When a saved datamodel from a migration in the migrations table is no longer valid.
    #[error("The migration contains an invalid schema.\n{}", render_datamodel_error(.0, Some(.1)))]
    InvalidPersistedDatamodel(datamodel::error::ErrorCollection, String),

    /// Failed to render a prisma schema to a string.
    #[error("Failed to render the schema to a string ({0:?})")]
    DatamodelRenderingError(datamodel::error::ErrorCollection),

    /// Errors from the connector.
    #[error("Connector error. (error: {0})")]
    ConnectorError(
        #[source]
        #[from]
        ConnectorError,
    ),

    /// Generic unspecified errors.
    #[error("Generic error. (error: {0})")]
    Generic(#[source] anyhow::Error),

    /// Error in command input.
    #[error("Error in command input. (error: {0})")]
    Input(#[source] anyhow::Error),
}

fn render_datamodel_error(err: &datamodel::error::ErrorCollection, schema: Option<&String>) -> String {
    match schema {
        Some(schema) => err.to_pretty_string("virtual_schema.prisma", schema),
        None => format!("Datamodel error in schema that could not be rendered. {}", err),
    }
}

impl From<ListMigrationsError> for CommandError {
    fn from(err: ListMigrationsError) -> Self {
        CommandError::Generic(err.into())
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
            .map_err(|err| CommandError::ProducedBadDatamodel(err))
            .unwrap_err();

        assert_eq!(
            err.to_string(),
            "The migration produced an invalid schema.\nDatamodel error in schema that could not be rendered. Type \"Post\" is neither a built-in type, nor refers to another model, custom type, or enum."
        )
    }
}
