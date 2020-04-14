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
    #[error("The migration produced an invalid schema.\n{}", render_datamodel_error(.0, None))]
    ProducedBadDatamodel(datamodel::error::ErrorCollection),

    /// When a saved datamodel from a migration in the migrations table is no longer valid.
    #[error("The migration contains an invalid schema.\n{}", render_datamodel_error(.0, Some(.1)))]
    InvalidPersistedDatamodel(datamodel::error::ErrorCollection, String),

    #[error("Failed to render the schema to a string  ({0:?})")]
    DatamodelRenderingError(datamodel::error::ErrorCollection),

    #[error("Initialization error. (error: {0})")]
    InitializationError(#[source] anyhow::Error),

    #[error("Connector error. (error: {0})")]
    ConnectorError(
        #[source]
        #[from]
        ConnectorError,
    ),

    #[error("Generic error. (error: {0})")]
    Generic(#[source] anyhow::Error),

    #[error("Error in command input. (error: {0})")]
    Input(#[source] anyhow::Error),
}

fn render_datamodel_error(err: &datamodel::error::ErrorCollection, schema: Option<&String>) -> String {
    match schema {
        Some(schema) => err.to_pretty_string("virtual_schema.prisma", schema),
        None => format!("Datamodel error in schema that could not be rendered. {}", err),
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
            "The migration produced an invalid schema.\nDatamodel error in schema that could not be rendered. Type \"Post\" is neither a built-in type, nor refers to another model, custom type, or enum. (span: Span { start: 76, end: 82 })\n"
        )
    }
}
