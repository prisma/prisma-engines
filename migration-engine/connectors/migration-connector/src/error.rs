use failure::{Error, Fail};

#[derive(Debug, Fail)]
pub enum ConnectorError {
    #[fail(display = "{}", _0)]
    Generic(Error),

    #[fail(display = "Error querying the database: {}", _0)]
    QueryError(Error),

    #[fail(display = "Database '{}' does not exist", db_name)]
    DatabaseDoesNotExist { db_name: String },

    #[fail(display = "Access denied to database '{}'", db_name)]
    DatabaseAccessDenied { db_name: String },

    #[fail(display = "Database '{}' already exists", db_name)]
    DatabaseAlreadyExists { db_name: String },

    #[fail(display = "Could not create the database. {}", explanation)]
    DatabaseCreationFailed { explanation: String },

    #[fail(display = "Authentication failed for user '{}'", user)]
    AuthenticationFailed { user: String },

    #[fail(display = "Connect timed out")]
    ConnectTimeout,

    #[fail(display = "Operation timed out")]
    Timeout,

    #[fail(display = "Error opening a TLS connection. {}", message)]
    TlsError { message: String },
}

impl From<quaint::error::Error> for ConnectorError {
    fn from(e: quaint::error::Error) -> Self {
        match e {
            quaint::error::Error::DatabaseDoesNotExist { db_name } => Self::DatabaseDoesNotExist { db_name },
            quaint::error::Error::DatabaseAccessDenied { db_name } => Self::DatabaseAccessDenied { db_name },
            quaint::error::Error::DatabaseAlreadyExists { db_name } => Self::DatabaseAlreadyExists { db_name },
            quaint::error::Error::AuthenticationFailed { user } => Self::AuthenticationFailed { user },
            quaint::error::Error::ConnectTimeout => Self::ConnectTimeout,
            quaint::error::Error::Timeout => Self::Timeout,
            quaint::error::Error::TlsError { message } => Self::TlsError { message },
            e => ConnectorError::QueryError(e.into()),
        }
    }
}
