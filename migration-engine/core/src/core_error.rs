use migration_connector::{ConnectorError, ListMigrationsError};
use std::{error::Error as StdError, fmt::Display};
use user_facing_errors::KnownError;

use crate::migration::datamodel_calculator::CalculatorError;

/// The result type for migration engine commands.
pub type CoreResult<T> = Result<T, CoreError>;

/// The top-level error type for migration engine commands.
#[derive(Debug)]
pub enum CoreError {
    /// When there was a bad datamodel as part of the input.
    ReceivedBadDatamodel(String),

    /// When a datamodel from a generated AST is wrong. This is basically an internal error.
    ProducedBadDatamodel(datamodel::error::ErrorCollection),

    /// When a saved datamodel from a migration in the migrations table is no longer valid.
    InvalidPersistedDatamodel(datamodel::error::ErrorCollection, String),

    /// Failed to render a prisma schema to a string.
    DatamodelRenderingError(datamodel::error::ErrorCollection),

    /// Errors from the connector.
    ConnectorError(ConnectorError),

    /// Generic unspecified errors.
    Generic(anyhow::Error),

    /// Error in command input.
    Input(anyhow::Error),
}

impl Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::ReceivedBadDatamodel(err) => write!(f, "{}", err),
            CoreError::ProducedBadDatamodel(err) => write!(
                f,
                "The migration produced an invalid schema.\n{}",
                render_datamodel_error(err, None)
            ),
            CoreError::InvalidPersistedDatamodel(err, schema) => write!(
                f,
                "The migration contains an invalid schema.\n{}",
                render_datamodel_error(err, Some(schema))
            ),
            CoreError::DatamodelRenderingError(err) => write!(f, "Failed to render the schema to a string ({:?})", err),
            CoreError::ConnectorError(err) => write!(f, "Connector error: {}", err),
            CoreError::Generic(src) => write!(f, "Generic error: {}", src),
            CoreError::Input(src) => write!(f, "Error in command input: {}", src),
        }
    }
}

impl StdError for CoreError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            CoreError::ReceivedBadDatamodel(_) => None,
            CoreError::ProducedBadDatamodel(_) => None,
            CoreError::InvalidPersistedDatamodel(_, _) => None,
            CoreError::DatamodelRenderingError(_) => None,
            CoreError::ConnectorError(err) => Some(err),
            CoreError::Generic(err) => Some(err.as_ref()),
            CoreError::Input(err) => Some(err.as_ref()),
        }
    }
}

impl CoreError {
    /// Render to an `user_facing_error::Error`.
    pub fn render_user_facing(self) -> user_facing_errors::Error {
        match self {
            CoreError::ConnectorError(err) => err.to_user_facing(),
            CoreError::ReceivedBadDatamodel(full_error) => {
                KnownError::new(user_facing_errors::common::SchemaParserError { full_error }).into()
            }
            crate_error => user_facing_errors::Error::from_dyn_error(&crate_error),
        }
    }
}

fn render_datamodel_error(err: &datamodel::error::ErrorCollection, schema: Option<&String>) -> String {
    match schema {
        Some(schema) => err.to_pretty_string("virtual_schema.prisma", schema),
        None => format!("Datamodel error in schema that could not be rendered. {}", err),
    }
}

impl From<ConnectorError> for CoreError {
    fn from(err: ConnectorError) -> Self {
        CoreError::ConnectorError(err)
    }
}

impl From<ListMigrationsError> for CoreError {
    fn from(err: ListMigrationsError) -> Self {
        CoreError::Generic(err.into())
    }
}

impl From<CalculatorError> for CoreError {
    fn from(error: CalculatorError) -> Self {
        CoreError::Generic(error.into())
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
            .map_err(|err| CoreError::ProducedBadDatamodel(err))
            .unwrap_err();

        assert_eq!(
            err.to_string(),
            "The migration produced an invalid schema.\nDatamodel error in schema that could not be rendered. Type \"Post\" is neither a built-in type, nor refers to another model, custom type, or enum."
        )
    }
}
