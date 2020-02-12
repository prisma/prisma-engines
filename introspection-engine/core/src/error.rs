use crate::command_error::CommandError;
use datamodel::error::ErrorCollection;
use introspection_connector::ConnectorError;
use thiserror::Error;

pub type CoreResult<T> = Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error in connector: {0}")]
    ConnectorError(ConnectorError),

    #[error("Failure during an introspection command: {0}")]
    CommandError(CommandError),

    #[error("Error in datamodel: {:?}", .0)]
    DatamodelError(ErrorCollection),
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

impl From<datamodel::error::ErrorCollection> for Error {
    fn from(e: datamodel::error::ErrorCollection) -> Self {
        Error::DatamodelError(e)
    }
}
