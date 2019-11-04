use failure::{Error, Fail};

#[derive(Debug, Fail)]
pub enum ConnectorError {
    #[fail(display = "{}", _0)]
    Generic(Error),

    #[fail(display = "Error querying the database: {}", _0)]
    QueryError(Error),

    #[fail(display = "Database '{}' does not exist", db_name)]
    DatabaseDoesNotExist { db_name: String, database_location: String },

    #[fail(display = "Access denied to database '{}'", database_name)]
    DatabaseAccessDenied {
        database_name: String,
        database_user: String,
    },

    #[fail(display = "Database '{}' already exists", db_name)]
    DatabaseAlreadyExists {
        db_name: String,
        database_host: String,
        database_port: u16,
    },

    #[fail(display = "Could not create the database. {}", explanation)]
    DatabaseCreationFailed { explanation: String },

    #[fail(display = "Authentication failed for user '{}'", user)]
    AuthenticationFailed { user: String, host: String },

    #[fail(display = "The database URL is not valid")]
    InvalidDatabaseUrl,

    #[fail(display = "Failed to connect to the database at `{}`.", host)]
    ConnectionError {
        host: String,
        port: Option<u16>,
        #[fail(cause)]
        cause: Error,
    },

    #[fail(display = "Connect timed out")]
    ConnectTimeout,

    #[fail(display = "Operation timed out")]
    Timeout,

    #[fail(display = "Error opening a TLS connection. {}", message)]
    TlsError { message: String },
}
