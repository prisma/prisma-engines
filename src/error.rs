//! Error module
use std::{fmt, io, num};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum DatabaseConstraint {
    Fields(Vec<String>),
    Index(String),
    ForeignKey,
}

impl fmt::Display for DatabaseConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fields(fields) => write!(f, "({})", fields.join(",")),
            Self::Index(index) => index.fmt(f),
            Self::ForeignKey => "FOREIGN KEY".fmt(f),
        }
    }
}

#[derive(Debug, Error)]
/// The error types for database I/O, connection and query parameter
/// construction.
pub struct Error {
    kind: ErrorKind,
    original_code: Option<String>,
    original_message: Option<String>,
}

pub(crate) struct ErrorBuilder {
    kind: ErrorKind,
    original_code: Option<String>,
    original_message: Option<String>,
}

impl ErrorBuilder {
    pub(crate) fn set_original_code(&mut self, code: impl Into<String>) -> &mut Self {
        self.original_code = Some(code.into());
        self
    }

    pub(crate) fn set_original_message(&mut self, message: impl Into<String>) -> &mut Self {
        self.original_message = Some(message.into());
        self
    }

    pub(crate) fn build(self) -> Error {
        Error {
            kind: self.kind,
            original_code: self.original_code,
            original_message: self.original_message,
        }
    }
}

impl Error {
    pub(crate) fn builder(kind: ErrorKind) -> ErrorBuilder {
        ErrorBuilder {
            kind,
            original_code: None,
            original_message: None,
        }
    }

    /// The error code sent by the database, if available.
    pub fn original_code(&self) -> Option<&str> {
        self.original_code.as_ref().map(|s| s.as_str())
    }

    /// The original error message sent by the database, if available.
    pub fn original_message(&self) -> Option<&str> {
        self.original_message.as_ref().map(|s| s.as_str())
    }

    /// A more specific error type for matching.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Debug, Error)]
pub enum ErrorKind {
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

    #[error("Unique constraint failed: {}", constraint)]
    UniqueConstraintViolation { constraint: DatabaseConstraint },

    #[error("Null constraint failed: {}", constraint)]
    NullConstraintViolation { constraint: DatabaseConstraint },

    #[error("Foreign key constraint failed: {}", constraint)]
    ForeignKeyConstraintViolation { constraint: DatabaseConstraint },

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

    #[error("The value provided for column {:?} is too long.", column)]
    LengthMismatch { column: Option<String> },

    #[error("The provided arguments are not supported")]
    InvalidConnectionArguments,

    #[error("Error in an I/O operation: {0}")]
    IoError(io::Error),

    #[error("Connect timed out ({0})")]
    ConnectTimeout(String),

    #[error("Operation timed out ({0})")]
    Timeout(String),

    #[error("Error opening a TLS connection. {}", message)]
    TlsError { message: String },

    #[error("Value out of range error. {}", message)]
    ValueOutOfRange { message: String },

    #[cfg(feature = "serde-support")]
    #[error("Deserializing a ResultRow {:?}", _0)]
    FromRowError(serde::de::value::Error),
}

impl From<Error> for ErrorKind {
    fn from(e: Error) -> Self {
        e.kind
    }
}

impl From<num::TryFromIntError> for Error {
    fn from(_: num::TryFromIntError) -> Self {
        Self::builder(ErrorKind::ConversionError(
            "Couldn't convert an integer (possible overflow).",
        ))
        .build()
    }
}

#[cfg(feature = "pooled")]
impl From<mobc::Error<Error>> for Error {
    fn from(e: mobc::Error<Error>) -> Self {
        match e {
            mobc::Error::Inner(e) => e,
            mobc::Error::Timeout => Error::builder(ErrorKind::Timeout("mobc timeout".into())).build(),
            e @ mobc::Error::BadConn => Error::builder(ErrorKind::ConnectionError(Box::new(e))).build(),
        }
    }
}

#[cfg(any(feature = "postgresql", feature = "mysql"))]
impl From<tokio::time::Elapsed> for Error {
    fn from(_: tokio::time::Elapsed) -> Self {
        Error::builder(ErrorKind::Timeout("tokio timeout".into())).build()
    }
}

impl From<url::ParseError> for Error {
    fn from(_: url::ParseError) -> Error {
        let kind = ErrorKind::DatabaseUrlIsInvalid("Error parsing database connection string.".to_string());
        Error::builder(kind).build()
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::builder(ErrorKind::IoError(e)).build()
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_: std::string::FromUtf8Error) -> Error {
        Error::builder(ErrorKind::ConversionError("Couldn't convert data to UTF-8")).build()
    }
}
