use std::fmt;
use crate::{CoreError, QueryValidationError, QueryGraphError};
use connector::error::ConnectorError;

#[derive(Debug)]
pub enum QueryExecutionError {
    EnvVarNotFound(String),
    InvalidQuery(QueryValidationError),
    ConnectorError(ConnectorError),
    TranslationError(String),
    InterpretationError(String),
    Generic(String),
}

impl fmt::Display for QueryExecutionError {
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

impl From<CoreError> for QueryExecutionError {
    fn from(e: CoreError) -> Self {
        QueryExecutionError::Generic(format!("{:?}", e))
    }
}

impl From<QueryValidationError> for QueryExecutionError {
    fn from(e: QueryValidationError) -> Self {
        QueryExecutionError::InvalidQuery(e)
    }
}

impl From<QueryGraphError> for QueryExecutionError {
    fn from(e: QueryGraphError) -> Self {
        QueryExecutionError::TranslationError(format!("{:?}", e))
    }
}

impl From<ConnectorError> for QueryExecutionError {
    fn from(e: ConnectorError) -> Self {
        QueryExecutionError::ConnectorError(e)
    }
}

// impl From<QueryExecutionError> for ConnectorError {
//     fn from(e: QueryExecutionError) -> Self {
//         ConnectorError::QueryError(e.into())
//     }
// }