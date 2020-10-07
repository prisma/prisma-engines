#![deny(missing_docs)]

//! Errors for the migration core.

use crate::commands::CommandError;
use datamodel::error::ErrorCollection;
use migration_connector::ConnectorError;
use std::{error::Error as StdError, fmt::Display};

/// Top-level result type for the migration core.
pub type CoreResult<T> = Result<T, Error>;

/// Top-level migration core error.
#[derive(Debug)]
pub enum Error {
    /// Error from a connector.
    ConnectorError(ConnectorError),

    /// Error from a migration command.
    CommandError(CommandError),

    /// Error from the datamodel parser.
    DatamodelError(ErrorCollection),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConnectorError(err) => write!(f, "Error in connector: {}", err),
            Error::CommandError(err) => write!(f, "Failure during a migration command: {}", err),
            Error::DatamodelError(err) => write!(f, "Error in datamodel: {}", err),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::ConnectorError(err) => Some(err),
            Error::CommandError(err) => Some(err),
            Error::DatamodelError(_) => None,
        }
    }
}

impl From<ConnectorError> for Error {
    fn from(err: ConnectorError) -> Self {
        Error::ConnectorError(err)
    }
}

impl From<CommandError> for Error {
    fn from(err: CommandError) -> Self {
        Error::CommandError(err)
    }
}

impl From<ErrorCollection> for Error {
    fn from(v: ErrorCollection) -> Self {
        Error::DatamodelError(v)
    }
}
