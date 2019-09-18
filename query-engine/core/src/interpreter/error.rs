use std::fmt;
use crate::{CoreError, QueryValidationError, QueryGraphError};
use connector::error::ConnectorError;
use prisma_models::DomainError;

#[derive(Debug)]
pub enum InterpreterError {
    EnvVarNotFound(String),
    TranslationError(String),
    DomainError(DomainError),
    InterpretationError(String),
    InvalidQuery(QueryValidationError),
    ConnectorError(ConnectorError),
    Generic(String),
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidQuery(e) => write!(f, "{}", e),
            _ => write!(
            f,
            "Error occurred during query execution:\n{:?}",
            self
        )
        }
    }
}

impl From<CoreError> for InterpreterError {
    fn from(e: CoreError) -> Self {
        InterpreterError::Generic(format!("{:?}", e))
    }
}

impl From<DomainError> for InterpreterError {
    fn from(e: DomainError) -> Self {
        InterpreterError::DomainError(e)
    }
}

impl From<QueryValidationError> for InterpreterError {
    fn from(e: QueryValidationError) -> Self {
        InterpreterError::InvalidQuery(e)
    }
}

impl From<QueryGraphError> for InterpreterError {
    fn from(e: QueryGraphError) -> Self {
        InterpreterError::TranslationError(format!("{:?}", e))
    }
}

impl From<ConnectorError> for InterpreterError {
    fn from(e: ConnectorError) -> Self {
        InterpreterError::ConnectorError(e)
    }
}

// impl From<InterpreterError> for ConnectorError {
//     fn from(e: InterpreterError) -> Self {
//         ConnectorError::QueryError(e.into())
//     }
// }