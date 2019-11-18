use failure::{Error, Fail};
use migration_connector::ConnectorError;

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
            error => ConnectorError::QueryError(error.into()),
        }
    }
}

impl From<quaint::error::Error> for SqlError {
    fn from(error: quaint::error::Error) -> Self {
        match error {
            quaint::error::Error::DatabaseDoesNotExist { db_name } => Self::DatabaseDoesNotExist { db_name },
            quaint::error::Error::DatabaseAlreadyExists { db_name } => Self::DatabaseAlreadyExists { db_name },
            quaint::error::Error::DatabaseAccessDenied { db_name } => Self::DatabaseAccessDenied { db_name },
            quaint::error::Error::AuthenticationFailed { user } => Self::AuthenticationFailed { user },
            quaint::error::Error::ConnectTimeout => Self::ConnectTimeout,
            quaint::error::Error::ConnectionError { .. } => Self::ConnectionError(error.into()),
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
