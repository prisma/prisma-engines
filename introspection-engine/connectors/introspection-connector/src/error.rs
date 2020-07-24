use std::fmt::Display;
use thiserror::Error;
use user_facing_errors::KnownError;

#[derive(Debug, Error)]
#[error("{}", kind)]
pub struct ConnectorError {
    /// An optional error already rendered for users in case the introspection core does not handle it.
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
            kind: ErrorKind::InvalidDatabaseUrl(format!("{} in `{}`)", err, url)),
        }
    }
}

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("{0}")]
    Generic(anyhow::Error),

    #[error("Error querying the database: {0}")]
    QueryError(anyhow::Error),

    #[error("Database '{}' does not exist", db_name)]
    DatabaseDoesNotExist { db_name: String },

    #[error("Access denied to database '{}'", database_name)]
    DatabaseAccessDenied { database_name: String },

    #[error("Database '{}' already exists", db_name)]
    DatabaseAlreadyExists { db_name: String },

    #[error("Could not create the database. {}", explanation)]
    DatabaseCreationFailed { explanation: String },

    #[error(
        "Could not introspect the database since the schema was inconsistent. {}",
        explanation
    )]
    DatabaseSchemaInconsistent { explanation: String },

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

    #[error("Operation timed out ({0})")]
    Timeout(String),

    #[error("Error opening a TLS connection. {}", message)]
    TlsError { message: String },
}
