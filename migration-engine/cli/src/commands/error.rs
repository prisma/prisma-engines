use migration_connector::ConnectorError;
use migration_core::CoreError;
use std::fmt::Display;
use tracing_error::SpanTrace;
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
    Unknown {
        error: ConnectorError,
        context: SpanTrace,
        exit_code: i32,
    },
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Known { error, exit_code: _ } => write!(f, "Known error: {:?}", error),
            CliError::Unknown {
                error,
                context,
                exit_code: _,
            } => write!(f, "{}\n{}", error, context),
        }
    }
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Known { exit_code, .. } => *exit_code,
            CliError::Unknown { exit_code, .. } => *exit_code,
        }
    }

    /// The errors spec error code, if applicable
    #[cfg(test)]
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

pub fn render_error(cli_error: CliError) -> user_facing_errors::Error {
    use user_facing_errors::UnknownError;

    match cli_error {
        CliError::Known { error, .. } => error.into(),
        other => UnknownError {
            message: format!("{}", other),
            backtrace: None,
        }
        .into(),
    }
}

impl From<ConnectorError> for CliError {
    fn from(err: ConnectorError) -> Self {
        let exit_code = exit_code(&err);
        let context = err.context().clone();

        match err.known_error() {
            Some(error) => CliError::Known {
                error: error.clone(),
                exit_code,
            },
            None => CliError::Unknown {
                error: err,
                exit_code,
                context,
            },
        }
    }
}

impl From<CoreError> for CliError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::ConnectorError(e) => e.into(),
            e => CliError::Unknown {
                error: ConnectorError::generic(e.into()),
                context: SpanTrace::capture(),
                exit_code: 255,
            },
        }
    }
}
