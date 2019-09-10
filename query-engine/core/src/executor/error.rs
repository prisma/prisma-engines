use std::fmt;
use crate::{CoreError, QueryValidationError};

#[derive(Debug)]
pub enum QueryExecutionError {
    EnvVarNotFound(String),
    InvalidQuery(QueryValidationError),
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