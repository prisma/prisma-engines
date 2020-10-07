use crate::command_error::CommandError;
use datamodel::messages::MessageCollection;
use introspection_connector::{ConnectorError, ErrorKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error in connector: {0}")]
    ConnectorError(ConnectorError),

    #[error("Failure during an introspection command: {0}")]
    CommandError(CommandError),

    #[error("Error in datamodel: {:?}", .0)]
    DatamodelError(MessageCollection),

    #[error("{}", _0)]
    InvalidDatabaseUrl(String),
}

impl From<ConnectorError> for Error {
    fn from(e: ConnectorError) -> Self {
        match e.kind {
            ErrorKind::InvalidDatabaseUrl(reason) => Self::InvalidDatabaseUrl(reason),
            _ => Error::ConnectorError(e),
        }
    }
}

impl From<CommandError> for Error {
    fn from(e: CommandError) -> Self {
        Error::CommandError(e)
    }
}

impl From<datamodel::messages::MessageCollection> for Error {
    fn from(e: datamodel::messages::MessageCollection) -> Self {
        Error::DatamodelError(e)
    }
}
