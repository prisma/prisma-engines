use failure::{err_msg, Error, Fail};
use migration_connector::ConnectorError;
use quaint::error::Error as QuaintError;

pub type SqlResult<T> = Result<T, SqlError>;

#[derive(Debug, Fail)]
pub enum SqlError {
    #[fail(display = "{}", _0)]
    Generic(String),

    #[fail(display = "Error connecting to the database {}", _0)]
    ConnectionError(Error),

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

    #[fail(display = "Unique constraint violation")]
    UniqueConstraintViolation { field_name: String },
}

impl SqlError {
    pub(crate) fn into_connector_error(self, connection_info: &super::ConnectionInfo) -> ConnectorError {
        match self {
            SqlError::DatabaseDoesNotExist { db_name } => ConnectorError::DatabaseDoesNotExist {
                db_name,
                database_location: connection_info.database_location(),
            },
            SqlError::DatabaseAccessDenied { db_name } => ConnectorError::DatabaseAccessDenied {
                database_name: db_name,
                database_user: connection_info
                    .username()
                    .expect("database access denied without user")
                    .into_owned(),
            },
            SqlError::DatabaseAlreadyExists { db_name } => ConnectorError::DatabaseAlreadyExists {
                db_name,
                database_host: connection_info.host().to_owned(),
                database_port: connection_info.port().expect("database port not applicable"),
            },
            SqlError::AuthenticationFailed { user } => ConnectorError::AuthenticationFailed {
                user,
                host: connection_info.host().to_owned(),
            },
            SqlError::ConnectTimeout => ConnectorError::ConnectTimeout,
            SqlError::Timeout => ConnectorError::Timeout,
            SqlError::TlsError { message } => ConnectorError::TlsError { message },
            SqlError::ConnectionError(err) => ConnectorError::ConnectionError {
                host: connection_info.host().to_owned(),
                port: connection_info.port(),
                cause: failure::err_msg(err),
            },
            SqlError::UniqueConstraintViolation { field_name } => {
                ConnectorError::UniqueConstraintViolation { field_name }
            }
            error => ConnectorError::QueryError(error.into()),
        }
    }
}

impl From<QuaintError> for SqlError {
    fn from(error: QuaintError) -> Self {
        match error {
            QuaintError::DatabaseDoesNotExist { db_name } => Self::DatabaseDoesNotExist { db_name },
            QuaintError::DatabaseAlreadyExists { db_name } => Self::DatabaseAlreadyExists { db_name },
            QuaintError::DatabaseAccessDenied { db_name } => Self::DatabaseAccessDenied { db_name },
            QuaintError::AuthenticationFailed { user } => Self::AuthenticationFailed { user },
            QuaintError::ConnectTimeout => Self::ConnectTimeout,
            QuaintError::ConnectionError { .. } => Self::ConnectionError(error.into()),
            QuaintError::Timeout => Self::Timeout,
            QuaintError::TlsError { message } => Self::TlsError { message },
            QuaintError::UniqueConstraintViolation { field_name } => Self::UniqueConstraintViolation { field_name },
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

impl From<url::ParseError> for SqlError {
    fn from(_: url::ParseError) -> Self {
        SqlError::ConnectionError(err_msg("Couldn't parse the connection string."))
    }
}
