//! Error module
use crate::connector::IsolationLevel;
use std::{borrow::Cow, fmt, io, num};
use thiserror::Error;

#[cfg(feature = "pooled")]
use std::time::Duration;

pub use crate::connector::mysql::MysqlError;
pub use crate::connector::postgres::PostgresError;
pub use crate::connector::sqlite::SqliteError;

#[derive(Debug, PartialEq, Eq)]
pub enum DatabaseConstraint {
    Fields(Vec<String>),
    Index(String),
    ForeignKey,
    CannotParse,
}

impl DatabaseConstraint {
    pub(crate) fn fields<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: ToString,
    {
        let fields = names.into_iter().map(|s| s.to_string()).collect();

        Self::Fields(fields)
    }
}

impl fmt::Display for DatabaseConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fields(fields) => write!(f, "({})", fields.join(",")),
            Self::Index(index) => index.fmt(f),
            Self::ForeignKey => "FOREIGN KEY".fmt(f),
            Self::CannotParse => "".fmt(f),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Name {
    Available(String),
    Unavailable,
}

impl Name {
    pub fn available(name: impl ToString) -> Self {
        Self::Available(name.to_string())
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available(name) => name.fmt(f),
            Self::Unavailable => write!(f, "(not available)"),
        }
    }
}

impl<T> From<Option<T>> for Name
where
    T: ToString,
{
    fn from(name: Option<T>) -> Self {
        match name {
            Some(name) => Self::available(name),
            None => Self::Unavailable,
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

pub struct ErrorBuilder {
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

    pub fn build(self) -> Error {
        Error {
            kind: self.kind,
            original_code: self.original_code,
            original_message: self.original_message,
        }
    }
}

impl Error {
    pub fn builder(kind: ErrorKind) -> ErrorBuilder {
        ErrorBuilder {
            kind,
            original_code: None,
            original_message: None,
        }
    }

    /// The error code sent by the database, if available.
    pub fn original_code(&self) -> Option<&str> {
        self.original_code.as_deref()
    }

    /// The original error message sent by the database, if available.
    pub fn original_message(&self) -> Option<&str> {
        self.original_message.as_deref()
    }

    /// A more specific error type for matching.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Determines if the error was associated with closed connection.
    pub fn is_closed(&self) -> bool {
        matches!(self.kind, ErrorKind::ConnectionClosed)
    }

    // Builds an error from a raw error coming from the connector
    pub fn raw_connector_error(status: String, reason: String) -> Error {
        Error::builder(ErrorKind::RawConnectorError { status, reason }).build()
    }

    // Builds an error from an externally stored error
    pub fn external_error(error_id: i32) -> Error {
        Error::builder(ErrorKind::ExternalError(error_id)).build()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("Error in the underlying connector ({}): {}", status, reason)]
    RawConnectorError { status: String, reason: String },

    #[error("Error querying the database: {}", _0)]
    QueryError(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Invalid input provided to query: {}", _0)]
    QueryInvalidInput(String),

    #[error("Database does not exist: {}", db_name)]
    DatabaseDoesNotExist { db_name: Name },

    #[error("Access denied to database {}", db_name)]
    DatabaseAccessDenied { db_name: Name },

    #[error("Database already exists {}", db_name)]
    DatabaseAlreadyExists { db_name: Name },

    #[error("Authentication failed for user {}", user)]
    AuthenticationFailed { user: Name },

    #[error("Query returned no data")]
    NotFound,

    #[error("No such table: {}", table)]
    TableDoesNotExist { table: Name },

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

    #[error("Error accessing result set, column not found: {}", column)]
    ColumnNotFound { column: Name },

    #[error("Error accessing result set, type mismatch, expected: {}", _0)]
    ResultTypeMismatch(&'static str),

    #[error("Error parsing connection string: {}", _0)]
    DatabaseUrlIsInvalid(String),

    #[error("Conversion failed: {}", _0)]
    ConversionError(Cow<'static, str>),

    #[error("The value provided for column {:?} is too long.", column)]
    LengthMismatch { column: Name },

    #[error("The provided arguments are not supported")]
    InvalidConnectionArguments,

    #[error("Error in an I/O operation: {0}")]
    IoError(io::Error),

    #[error("Timed out when connecting to the database.")]
    ConnectTimeout,

    #[error("The server terminated the connection.")]
    ConnectionClosed,

    #[error(
        "Timed out fetching a connection from the pool (connection limit: {}, in use: {}, pool timeout {})",
        max_open,
        in_use,
        timeout
    )]
    PoolTimeout { max_open: u64, in_use: u64, timeout: u64 },

    #[error("The connection pool has been closed")]
    PoolClosed {},

    #[error("Timed out during query execution.")]
    SocketTimeout,

    #[error("Error opening a TLS connection. {}", message)]
    TlsError { message: String },

    #[error("Value out of range error. {}", message)]
    ValueOutOfRange { message: String },

    #[error(
        "Incorrect number of parameters given to a statement. Expected {}: got: {}.",
        expected,
        actual
    )]
    IncorrectNumberOfParameters { expected: usize, actual: usize },

    #[error("Transaction was already closed: {}", _0)]
    TransactionAlreadyClosed(String),

    #[error("Transaction write conflict")]
    TransactionWriteConflict,

    #[error("ROLLBACK statement has no corresponding BEGIN statement")]
    RollbackWithoutBegin,

    #[error("Invalid isolation level: {}", _0)]
    InvalidIsolationLevel(String),

    #[error("Error creating UUID, {}", _0)]
    UUIDError(String),

    #[error("Cannot find a FULLTEXT index to use for the search")]
    MissingFullTextSearchIndex,

    #[error("Column type '{}' could not be deserialized from the database.", column_type)]
    UnsupportedColumnType { column_type: String },

    #[error("External error id#{}", _0)]
    ExternalError(i32),
}

impl ErrorKind {
    #[cfg(feature = "mysql-native")]
    pub(crate) fn value_out_of_range(msg: impl Into<String>) -> Self {
        Self::ValueOutOfRange { message: msg.into() }
    }

    pub(crate) fn conversion(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::ConversionError(msg.into())
    }

    #[allow(dead_code)]
    pub(crate) fn database_url_is_invalid(msg: impl Into<String>) -> Self {
        Self::DatabaseUrlIsInvalid(msg.into())
    }

    #[cfg(feature = "pooled")]
    pub(crate) fn pool_timeout(max_open: u64, in_use: u64, timeout: Duration) -> Self {
        Self::PoolTimeout {
            max_open,
            in_use,
            timeout: timeout.as_secs(),
        }
    }

    pub fn invalid_isolation_level(isolation_level: &IsolationLevel) -> Self {
        Self::InvalidIsolationLevel(isolation_level.to_string())
    }
}

impl From<Error> for ErrorKind {
    fn from(e: Error) -> Self {
        e.kind
    }
}

impl From<bigdecimal::ParseBigDecimalError> for Error {
    fn from(e: bigdecimal::ParseBigDecimalError) -> Self {
        let kind = ErrorKind::conversion(format!("{e}"));
        Self::builder(kind).build()
    }
}

impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::builder(ErrorKind::conversion("Malformed JSON data.")).build()
    }
}

impl From<std::fmt::Error> for Error {
    fn from(_: std::fmt::Error) -> Self {
        Self::builder(ErrorKind::conversion("Problems writing AST into a query string.")).build()
    }
}

impl From<num::TryFromIntError> for Error {
    fn from(_: num::TryFromIntError) -> Self {
        Self::builder(ErrorKind::conversion(
            "Couldn't convert an integer (possible overflow).",
        ))
        .build()
    }
}

impl From<connection_string::Error> for Error {
    fn from(err: connection_string::Error) -> Error {
        Self::builder(ErrorKind::DatabaseUrlIsInvalid(err.to_string())).build()
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Error {
        let kind = ErrorKind::DatabaseUrlIsInvalid(e.to_string());
        Error::builder(kind).build()
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::builder(ErrorKind::IoError(e)).build()
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(_e: std::num::ParseIntError) -> Error {
        Error::builder(ErrorKind::conversion("Couldn't convert data to an integer")).build()
    }
}

impl From<std::str::ParseBoolError> for Error {
    fn from(_e: std::str::ParseBoolError) -> Error {
        Error::builder(ErrorKind::conversion("Couldn't convert data to a boolean")).build()
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_: std::string::FromUtf8Error) -> Error {
        Error::builder(ErrorKind::conversion("Couldn't convert data to UTF-8")).build()
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Self {
        Error::builder(ErrorKind::conversion(format!(
            "Couldn't convert data to std::net::IpAddr: {e}"
        )))
        .build()
    }
}

impl From<uuid::Error> for Error {
    fn from(e: uuid::Error) -> Self {
        Error::builder(ErrorKind::UUIDError(format!("{e}"))).build()
    }
}
