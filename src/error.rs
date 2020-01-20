//! Error module
use thiserror::Error;
use std::io;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error querying the database: {}", _0)]
    QueryError(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Database '{}' does not exist.", db_name)]
    DatabaseDoesNotExist { db_name: String },

    #[error("Access denied to database '{}'", db_name)]
    DatabaseAccessDenied { db_name: String },

    #[error("Database '{}' already exists", db_name)]
    DatabaseAlreadyExists { db_name: String },

    #[error("Authentication failed for user '{}'", user)]
    AuthenticationFailed { user: String },

    #[error("Query returned no data")]
    NotFound,

    #[error("Unique constraint failed: {}", field_name)]
    UniqueConstraintViolation { field_name: String },

    #[error("Null constraint failed: {}", field_name)]
    NullConstraintViolation { field_name: String },

    #[error("Error creating a database connection.")]
    ConnectionError(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Error reading the column value: {}", _0)]
    ColumnReadFailure(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Error accessing result set, index out of bounds: {}", _0)]
    ResultIndexOutOfBounds(usize),

    #[error("Error accessing result set, column not found: {}", _0)]
    ColumnNotFound(String),

    #[error("Error accessing result set, type mismatch, expected: {}", _0)]
    ResultTypeMismatch(&'static str),

    #[error("The specified database url {} is invalid", _0)]
    DatabaseUrlIsInvalid(String),

    #[error("Conversion failed: {}", _0)]
    ConversionError(&'static str),

    #[error("The provided arguments are not supported")]
    InvalidConnectionArguments,

    #[error("Error in an I/O operation")]
    IoError(io::Error),

    #[error("Connect timed out")]
    ConnectTimeout,

    #[error("Operation timed out")]
    Timeout,

    #[error("Error opening a TLS connection. {}", message)]
    TlsError { message: String },
}

#[cfg(feature = "pooled")]
impl From<mobc::Error<Error>> for Error {
    fn from(e: mobc::Error<Error>) -> Self {
        match e {
            mobc::Error::Inner(e) => e,
            mobc::Error::Timeout => Self::Timeout,
        }
    }
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
impl From<tokio::time::Elapsed> for Error {
    fn from(_: tokio::time::Elapsed) -> Self {
        Self::Timeout
    }
}

impl From<url::ParseError> for Error {
    fn from(_: url::ParseError) -> Error {
        Error::DatabaseUrlIsInvalid("Error parsing database connection string.".to_string())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::IoError(e)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_: std::string::FromUtf8Error) -> Error {
        Error::ConversionError("Couldn't convert data to UTF-8")
    }
}
