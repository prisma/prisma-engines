use crate::migrations_directory::ReadMigrationScriptError;
use std::{error::Error as StdError, fmt::Display};
use tracing_error::SpanTrace;
use user_facing_errors::{KnownError, UserFacingError};

#[derive(Debug)]
pub struct ConnectorError {
    /// An optional error already rendered for users in case the migration core does not handle it.
    pub user_facing_error: Option<KnownError>,
    /// The error information for internal use.
    pub kind: ErrorKind,
    /// See the tracing-error docs.
    pub context: SpanTrace,
}

impl Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n{}", self.kind, self.context)
    }
}

impl StdError for ConnectorError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.kind)
    }
}

impl ConnectorError {
    pub fn from_kind(kind: ErrorKind) -> Self {
        ConnectorError {
            user_facing_error: None,
            kind,
            context: SpanTrace::capture(),
        }
    }

    pub fn generic(error: anyhow::Error) -> Self {
        ConnectorError {
            user_facing_error: None,
            kind: ErrorKind::Generic(error),
            context: SpanTrace::capture(),
        }
    }

    pub fn into_migration_failed(self, migration_name: String) -> Self {
        let context = self.context.clone();
        let user_facing_error = user_facing_errors::migration_engine::MigrationDoesNotApplyCleanly {
            migration_name,
            inner_error: self
                .user_facing_error
                .clone()
                .map(user_facing_errors::Error::new_known)
                .unwrap_or_else(|| user_facing_errors::Error::new_non_panic_with_current_backtrace(self.to_string())),
        };

        ConnectorError {
            user_facing_error: Some(KnownError::new(user_facing_error)),
            kind: ErrorKind::Generic(self.into()),
            context,
        }
    }

    pub fn query_error(error: anyhow::Error) -> Self {
        let kind = ErrorKind::QueryError(error);

        ConnectorError {
            user_facing_error: None,
            kind,
            context: SpanTrace::capture(),
        }
    }

    pub fn url_parse_error(err: impl Display, url: &str) -> Self {
        ConnectorError {
            user_facing_error: None,
            kind: ErrorKind::InvalidDatabaseUrl(format!("{} in `{}`)", err, url)),
            context: SpanTrace::capture(),
        }
    }

    pub fn user_facing_error<T: UserFacingError>(err: T) -> Self {
        let kind = ErrorKind::Generic(anyhow::anyhow!("{}", err.message()));
        ConnectorError {
            user_facing_error: Some(KnownError::new(err)),
            kind,
            context: SpanTrace::capture(),
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Generic(anyhow::Error),
    QueryError(anyhow::Error),
    DatabaseDoesNotExist { db_name: String },
    DatabaseAccessDenied { database_name: String },
    DatabaseAlreadyExists { db_name: String },
    DatabaseCreationFailed { explanation: String },
    AuthenticationFailed { user: String },
    InvalidDatabaseUrl(String),
    ConnectionError { host: String, cause: anyhow::Error },
    ConnectTimeout,
    Timeout,
    TlsError { message: String },
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Generic(err) => err.fmt(f),
            ErrorKind::QueryError(err) => write!(f, "Error querying the database: {}", err),
            ErrorKind::DatabaseDoesNotExist { db_name } => write!(f, "Database `{}` does not exist", db_name),
            ErrorKind::DatabaseAccessDenied { database_name } => {
                write!(f, "Access denied to database `{}`", database_name)
            }
            ErrorKind::DatabaseAlreadyExists { db_name } => write!(f, "Database '{}' already exists", db_name),
            ErrorKind::DatabaseCreationFailed { explanation } => {
                write!(f, "Could not create the database. {}", explanation)
            }
            ErrorKind::AuthenticationFailed { user } => write!(f, "Authentication failed for user '{}'", user),
            ErrorKind::InvalidDatabaseUrl(err) => err.fmt(f),
            ErrorKind::ConnectionError { host, cause: _ } => {
                write!(f, "Failed to connect to the database at `{}`.", host)
            }
            ErrorKind::ConnectTimeout => "Connection timed out".fmt(f),
            ErrorKind::Timeout => "Operation timed out".fmt(f),
            ErrorKind::TlsError { message } => write!(f, "Error opening a TLS connection. {}", message),
        }
    }
}

impl StdError for ErrorKind {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ErrorKind::Generic(err) => Some(err.as_ref()),
            ErrorKind::QueryError(err) => Some(err.as_ref()),
            ErrorKind::DatabaseDoesNotExist { db_name: _ } => None,
            ErrorKind::DatabaseAccessDenied { database_name: _ } => None,
            ErrorKind::DatabaseAlreadyExists { db_name: _ } => None,
            ErrorKind::DatabaseCreationFailed { explanation: _ } => None,
            ErrorKind::AuthenticationFailed { user: _ } => None,
            ErrorKind::InvalidDatabaseUrl(_) => None,
            ErrorKind::ConnectionError { host: _, cause } => Some(cause.as_ref()),
            ErrorKind::ConnectTimeout => None,
            ErrorKind::Timeout => None,
            ErrorKind::TlsError { message: _ } => None,
        }
    }
}

impl From<ReadMigrationScriptError> for ConnectorError {
    fn from(err: ReadMigrationScriptError) -> Self {
        let context = err.1.clone();
        ConnectorError {
            user_facing_error: None,
            kind: ErrorKind::Generic(err.into()),
            context,
        }
    }
}
