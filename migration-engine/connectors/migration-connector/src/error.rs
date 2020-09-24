use crate::migrations_directory::ReadMigrationScriptError;
use std::fmt::Display;
use thiserror::Error;
use tracing_error::SpanTrace;
use user_facing_errors::KnownError;

#[derive(Debug, Error)]
#[error("{}\n{}", kind, context)]
pub struct ConnectorError {
    /// An optional error already rendered for users in case the migration core does not handle it.
    pub user_facing_error: Option<KnownError>,
    /// The error information for internal use.
    pub kind: ErrorKind,
    /// See the tracing-error docs.
    pub context: SpanTrace,
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
        let user_facing_error = self.user_facing_error.clone();

        ConnectorError {
            user_facing_error,
            kind: ErrorKind::MigrationFailedToApply {
                migration_name,
                error: self.into(),
            },
            context,
        }
    }

    pub fn url_parse_error(err: impl Display, url: &str) -> Self {
        ConnectorError {
            user_facing_error: None,
            kind: ErrorKind::InvalidDatabaseUrl(format!("{} in `{}`)", err, url)),
            context: SpanTrace::capture(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error(transparent)]
    Generic(anyhow::Error),

    #[error("Error querying the database: {0}")]
    QueryError(#[source] anyhow::Error),

    #[error("Database '{}' does not exist", db_name)]
    DatabaseDoesNotExist { db_name: String },

    #[error("Access denied to database '{}'", database_name)]
    DatabaseAccessDenied { database_name: String },

    #[error("Database '{}' already exists", db_name)]
    DatabaseAlreadyExists { db_name: String },

    #[error("Could not create the database. {}", explanation)]
    DatabaseCreationFailed { explanation: String },

    #[error("Authentication failed for user '{}'", user)]
    AuthenticationFailed { user: String },

    #[error("{}", _0)]
    InvalidDatabaseUrl(String),

    #[error("Failed to connect to the database at `{}`.", host)]
    ConnectionError {
        host: String,
        #[source]
        cause: anyhow::Error,
    },

    #[error("Connect timed out")]
    ConnectTimeout,

    #[error(
        "Migration `{}` failed to apply cleanly to a temporary database. {}",
        migration_name,
        error
    )]
    MigrationFailedToApply {
        migration_name: String,
        error: anyhow::Error,
    },

    #[error("Operation timed out")]
    Timeout,

    #[error("Error opening a TLS connection. {}", message)]
    TlsError { message: String },

    #[error("Unique constraint violation.")]
    UniqueConstraintViolation { field_name: String },
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
