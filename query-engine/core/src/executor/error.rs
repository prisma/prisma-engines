use std::fmt;
use crate::CoreError;

#[derive(Debug)]
pub enum QueryExecutionError {
    InvalidEnv(String),
    Generic(String),
}

impl fmt::Display for QueryExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error occurred during query execution:\n{:?}",
            self
        )
    }
}

impl From<CoreError> for QueryExecutionError {
    fn from(e: CoreError) -> Self {
        QueryExecutionError::Generic(format!("{:?}", e))
    }
}