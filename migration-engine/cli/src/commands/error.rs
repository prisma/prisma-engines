use migration_connector::*;
use migration_core::{commands::CommandError, error::Error as CoreError};
use std::fmt::Display;
use tracing_error::SpanTrace;

#[derive(Debug)]
pub enum CliError {
    Known {
        error: user_facing_errors::KnownError,
        exit_code: i32,
    },
    Unknown {
        error: migration_connector::ErrorKind,
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

pub fn exit_code(error_kind: &migration_connector::ErrorKind) -> i32 {
    match error_kind {
        ErrorKind::DatabaseDoesNotExist { .. } => 1,
        ErrorKind::DatabaseAccessDenied { .. } => 2,
        ErrorKind::AuthenticationFailed { .. } => 3,
        ErrorKind::ConnectTimeout | ErrorKind::Timeout => 4,
        ErrorKind::DatabaseAlreadyExists { .. } => 5,
        ErrorKind::TlsError { .. } => 6,
        _ => 255,
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
    fn from(e: ConnectorError) -> Self {
        let ConnectorError {
            user_facing_error,
            kind: error_kind,
            context,
        } = e;

        let exit_code = exit_code(&error_kind);

        match user_facing_error {
            Some(error) => CliError::Known { error, exit_code },
            None => CliError::Unknown {
                error: error_kind,
                exit_code,
                context,
            },
        }
    }
}

impl From<CoreError> for CliError {
    fn from(e: migration_core::error::Error) -> Self {
        match e {
            CoreError::ConnectorError(e) | CoreError::CommandError(CommandError::ConnectorError(e)) => e.into(),
            e => CliError::Unknown {
                error: ErrorKind::Generic(e.into()),
                context: SpanTrace::capture(),
                exit_code: 255,
            },
        }
    }
}
