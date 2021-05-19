use migration_connector::ConnectorError;
use std::fmt::Display;
use user_facing_errors::{
    common::DatabaseAccessDenied, common::DatabaseAlreadyExists, common::DatabaseDoesNotExist,
    common::DatabaseNotReachable, common::DatabaseTimeout, common::IncorrectDatabaseCredentials,
    common::TlsConnectionError, UserFacingError,
};

#[derive(Debug)]
pub enum CliError {
    Known {
        error: user_facing_errors::KnownError,
        exit_code: i32,
    },
    InvalidParameters {
        error: String,
        exit_code: i32,
    },
    Unknown {
        error: ConnectorError,
        exit_code: i32,
    },
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Known { error, exit_code: _ } => f.write_str(&error.message),
            CliError::InvalidParameters { error, .. } => write!(f, "Invalid parameters: {}", error),
            CliError::Unknown { error, exit_code: _ } => Display::fmt(error, f),
        }
    }
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Known { exit_code, .. } => *exit_code,
            CliError::Unknown { exit_code, .. } => *exit_code,
            CliError::InvalidParameters { exit_code, .. } => *exit_code,
        }
    }

    pub fn invalid_parameters<S: ToString>(error: S) -> Self {
        Self::InvalidParameters {
            error: error.to_string(),
            exit_code: 255,
        }
    }

    /// The errors spec error code, if applicable
    pub(crate) fn error_code(&self) -> Option<&str> {
        match self {
            CliError::Known {
                error: user_facing_errors::KnownError { error_code, .. },
                ..
            } => Some(error_code),
            _ => None,
        }
    }
}

pub fn exit_code(error: &migration_connector::ConnectorError) -> i32 {
    match error.error_code() {
        Some(DatabaseDoesNotExist::ERROR_CODE) => 1,
        Some(DatabaseAccessDenied::ERROR_CODE) => 2,
        Some(IncorrectDatabaseCredentials::ERROR_CODE) => 3,
        Some(DatabaseTimeout::ERROR_CODE) | Some(DatabaseNotReachable::ERROR_CODE) => 4,
        Some(DatabaseAlreadyExists::ERROR_CODE) => 5,
        Some(TlsConnectionError::ERROR_CODE) => 6,
        Some(_) | None => 255,
    }
}

impl From<ConnectorError> for CliError {
    fn from(err: ConnectorError) -> Self {
        let exit_code = exit_code(&err);

        match err.known_error() {
            Some(error) => CliError::Known {
                error: error.clone(),
                exit_code,
            },
            None => CliError::Unknown { error: err, exit_code },
        }
    }
}
