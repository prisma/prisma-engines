use failure::{Error as FError, Fail};
use std::io;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Error querying the database: {}", _0)]
    QueryError(FError),

    #[fail(display = "Database '{}' does not exist.", db_name)]
    DatabaseDoesNotExist { db_name: String },

    #[fail(display = "Access denied to database '{}'", db_name)]
    DatabaseAccessDenied { db_name: String },

    #[fail(display = "Database '{}' already exists", db_name)]
    DatabaseAlreadyExists { db_name: String },

    #[fail(display = "Authentication failed for user '{}'", user)]
    AuthenticationFailed { user: String },

    #[fail(display = "Query returned no data")]
    NotFound,

    #[fail(display = "Unique constraint failed: {}", field_name)]
    UniqueConstraintViolation { field_name: String },

    #[fail(display = "Null constraint failed: {}", field_name)]
    NullConstraintViolation { field_name: String },

    #[fail(display = "Error creating a database connection.")]
    ConnectionError(FError),

    #[fail(display = "Error reading the column value: {}", _0)]
    ColumnReadFailure(FError),

    #[fail(display = "Error accessing result set, index out of bounds: {}", _0)]
    ResultIndexOutOfBounds(usize),

    #[fail(display = "Error accessing result set, column not found: {}", _0)]
    ColumnNotFound(String),

    #[fail(
        display = "Error accessing result set, type mismatch, expected: {}",
        _0
    )]
    ResultTypeMismatch(&'static str),

    #[fail(display = "The specified database url {} is invalid", _0)]
    DatabaseUrlIsInvalid(String),

    #[fail(display = "Conversion failed: {}", _0)]
    ConversionError(&'static str),

    #[fail(display = "The provided arguments are not supported")]
    InvalidConnectionArguments,

    #[fail(display = "Error in an I/O operation")]
    IoError(FError),

    #[fail(display = "Connect timed out")]
    ConnectTimeout,

    #[fail(display = "Operation timed out")]
    Timeout,
}

#[cfg(any(
    feature = "mysql-16",
    feature = "postgresql-0_16",
    feature = "rusqlite-0_19"
))]
impl From<r2d2::Error> for Error {
    fn from(e: r2d2::Error) -> Error {
        Error::ConnectionError(e.into())
    }
}

impl From<url::ParseError> for Error {
    fn from(_: url::ParseError) -> Error {
        Error::DatabaseUrlIsInvalid("Error parsing database connection string.".to_string())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::IoError(e.into())
    }
}
