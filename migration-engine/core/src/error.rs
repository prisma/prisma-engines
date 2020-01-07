use crate::commands::CommandError;
use datamodel::error::ErrorCollection;
use migration_connector::ConnectorError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error in connector: {0}")]
    ConnectorError(ConnectorError),

    #[error("Failure during a migration command: {0}")]
    CommandError(CommandError),

    #[error("Error in datamodel: {:?}", .0)]
    DatamodelError(ErrorCollection),

    #[error("Error performing IO: {:?}", .0)]
    IOError(anyhow::Error),
}

impl From<ConnectorError> for Error {
    fn from(e: ConnectorError) -> Self {
        Error::ConnectorError(e)
    }
}

impl From<CommandError> for Error {
    fn from(e: CommandError) -> Self {
        Error::CommandError(e)
    }
}

impl From<ErrorCollection> for Error {
    fn from(e: ErrorCollection) -> Self {
        Error::DatamodelError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IOError(e.into())
    }
}
