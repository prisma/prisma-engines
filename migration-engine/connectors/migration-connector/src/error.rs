use anyhow::format_err;
use std::fmt::Display;
use thiserror::Error;
use user_facing_errors::KnownError;

#[derive(Debug, Error)]
#[error("{}", kind)]
pub struct ConnectorError {
    /// An optional error already rendered for users in case the migration core does not handle it.
    pub user_facing_error: Option<KnownError>,
    /// The error information for internal use.
    pub kind: ErrorKind,
}

impl ConnectorError {
    pub fn from_kind(kind: ErrorKind) -> Self {
        ConnectorError {
            user_facing_error: None,
            kind,
        }
    }

    pub fn url_parse_error(err: impl Display, url: &str) -> Self {
        ConnectorError {
            user_facing_error: None,
            kind: ErrorKind::Generic(format_err!(
                "Could not parse the database connection string `{}`: {}",
                url,
                err
            )),
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

    #[error("The database URL is not valid")]
    InvalidDatabaseUrl,

    #[error("Failed to connect to the database at `{}`.", host)]
    ConnectionError {
        host: String,
        #[source]
        cause: anyhow::Error,
    },

    #[error("Connect timed out")]
    ConnectTimeout,

    #[error("Operation timed out")]
    Timeout,

    #[error("Error opening a TLS connection. {}", message)]
    TlsError { message: String },

    #[error("Unique constraint violation.")]
    UniqueConstraintViolation { field_name: String },
}
