use failure::{Error, Fail};
use migration_connector::ConnectorError;

pub type SqlResult<T> = Result<T, SqlError>;

#[derive(Debug, Fail)]
pub enum SqlError {
    #[fail(display = "{}", _0)]
    Generic(String),

    #[fail(display = "Error connecting to the database {}", _0)]
    ConnectionError(&'static str),

    #[fail(display = "Error querying the database: {}", _0)]
    QueryError(Error),

    #[fail(display = "Database '{}' does not exist", db_name)]
    DatabaseDoesNotExist { db_name: String },

    #[fail(display = "Access denied to database '{}'", db_name)]
    DatabaseAccessDenied { db_name: String },

    #[fail(display = "Database '{}' already exists", db_name)]
    DatabaseAlreadyExists { db_name: String },

    #[fail(display = "Authentication failed for user '{}'", user)]
    AuthenticationFailed { user: String },

    #[fail(display = "Connect timed out")]
    ConnectTimeout,

    #[fail(display = "Operation timed out")]
    Timeout,

    #[fail(display = "Error opening a TLS connection. {}", message)]
    TlsError { message: String },
}

impl From<SqlError> for ConnectorError {
    fn from(error: SqlError) -> Self {
        match error {
            SqlError::DatabaseDoesNotExist { db_name } => Self::DatabaseDoesNotExist { db_name },
            SqlError::DatabaseAccessDenied { db_name } => Self::DatabaseAccessDenied { db_name },
            SqlError::DatabaseAlreadyExists { db_name } => Self::DatabaseAlreadyExists { db_name },
            SqlError::AuthenticationFailed { user } => Self::AuthenticationFailed { user },
            SqlError::ConnectTimeout => Self::ConnectTimeout,
            SqlError::Timeout => Self::Timeout,
            SqlError::TlsError { message } => Self::TlsError { message },
            error => Self::QueryError(error.into()),
        }
    }
}

impl From<quaint::error::Error> for SqlError {
    fn from(error: quaint::error::Error) -> Self {
        match error {
            quaint::error::Error::DatabaseDoesNotExist { db_name } => Self::DatabaseDoesNotExist { db_name },
            quaint::error::Error::DatabaseAccessDenied { db_name } => Self::DatabaseAccessDenied { db_name },
            quaint::error::Error::AuthenticationFailed { user } => Self::AuthenticationFailed { user },
            quaint::error::Error::ConnectTimeout => Self::ConnectTimeout,
            quaint::error::Error::Timeout => Self::Timeout,
            quaint::error::Error::TlsError { message } => Self::TlsError { message },
            e => SqlError::QueryError(e.into()),
        }
    }
}

impl From<sql_schema_describer::SqlSchemaDescriberError> for SqlError {
    fn from(error: sql_schema_describer::SqlSchemaDescriberError) -> Self {
        SqlError::QueryError(error.into())
    }
}

impl From<String> for SqlError {
    fn from(error: String) -> Self {
        SqlError::Generic(error)
    }
}
