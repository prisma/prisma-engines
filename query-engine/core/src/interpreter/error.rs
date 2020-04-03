use crate::{CoreError, QueryGraphBuilderError, QueryGraphError};
use connector::error::ConnectorError;
use prisma_models::DomainError;
use std::fmt;

#[derive(Debug)]
pub enum InterpreterError {
    EnvVarNotFound(String),

    DomainError(DomainError),

    /// Expresses an error that ocurred during interpretation.
    ///
    /// The second field is an optional cause for this error.
    InterpretationError(String, Option<Box<InterpreterError>>),

    QueryGraphError(QueryGraphError),

    /// Wraps errors occurring during the query graph building stage.
    QueryGraphBuilderError(QueryGraphBuilderError),

    /// Wraps errors coming from the connector during execution.
    ConnectorError(ConnectorError),

    Generic(String),
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::QueryGraphBuilderError(e) => write!(f, "{:?}", e),
            _ => write!(f, "Error occurred during query execution:\n{:?}", self),
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

impl From<QueryGraphBuilderError> for InterpreterError {
    fn from(e: QueryGraphBuilderError) -> Self {
        InterpreterError::QueryGraphBuilderError(e)
    }
}

impl From<QueryGraphError> for InterpreterError {
    fn from(e: QueryGraphError) -> Self {
        InterpreterError::QueryGraphError(e)
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
