//! Remove this module when mongo opens up their connection string parsing.

use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("{kind}")]
pub struct Error {
    pub kind: ErrorKind,
}

#[derive(Clone, Debug, Error)]
pub enum ErrorKind {
    #[error("An invalid argument was provided: {message}")]
    InvalidArgument { message: String },
    #[error("{}", _0)]
    Other(mongodb::error::Error),
}

impl ErrorKind {
    pub(crate) fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: message.into(),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

impl From<mongodb::error::Error> for Error {
    fn from(e: mongodb::error::Error) -> Self {
        let kind = match &*e.kind {
            mongodb::error::ErrorKind::InvalidArgument { message, .. } => ErrorKind::invalid_argument(message),
            _ => ErrorKind::Other(e),
        };

        Error::from(kind)
    }
}
