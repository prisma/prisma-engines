use migration_connector::{ConnectorError, ErrorKind};
use quaint::error::{Error as QuaintError, DatabaseConstraint};
use thiserror::Error;
use user_facing_errors::quaint::render_quaint_error;

pub type SqlResult<T> = Result<T, SqlError>;

#[derive(Debug, Error)]
pub enum SqlError {
    #[error("{0}")]
    Generic(#[source] anyhow::Error),

    #[error("Error connecting to the database {cause}")]
    ConnectionError {
        #[source]
        cause: QuaintError,
    },

    #[error("Error querying the database: {}", _0)]
    QueryError(#[source] anyhow::Error),

    #[error("Database '{}' does not exist", db_name)]
    DatabaseDoesNotExist {
        db_name: String,
        #[source]
        cause: QuaintError,
    },

    #[error("Access denied to database '{}'", db_name)]
    DatabaseAccessDenied {
        db_name: String,
        #[source]
        cause: QuaintError,
    },

    #[error("Database '{}' already exists", db_name)]
    DatabaseAlreadyExists {
        db_name: String,
        #[source]
        cause: QuaintError,
    },

    #[error("Authentication failed for user '{}'", user)]
    AuthenticationFailed {
        user: String,
        #[source]
        cause: QuaintError,
    },

    #[error("Connect timed out")]
    ConnectTimeout(#[source] QuaintError),

    #[error("Operation timed out")]
    Timeout,

    #[error("Error opening a TLS connection. {}", cause)]
    TlsError {
        #[source]
        cause: QuaintError,
    },

    #[error("Unique constraint violation")]
    UniqueConstraintViolation {
        field_name: String,
        #[source]
        cause: QuaintError,
    },
}

impl SqlError {
    pub(crate) fn into_connector_error(self, connection_info: &super::ConnectionInfo) -> ConnectorError {
        match self {
            SqlError::DatabaseDoesNotExist { db_name, cause } => ConnectorError {
                user_facing_error: render_quaint_error(&cause, connection_info),
                kind: ErrorKind::DatabaseDoesNotExist { db_name },
            },
            SqlError::DatabaseAccessDenied { db_name, cause } => ConnectorError {
                user_facing_error: render_quaint_error(&cause, connection_info),
                kind: ErrorKind::DatabaseAccessDenied { database_name: db_name },
            },

            SqlError::DatabaseAlreadyExists { db_name, cause } => ConnectorError {
                user_facing_error: render_quaint_error(&cause, connection_info),
                kind: ErrorKind::DatabaseAlreadyExists { db_name },
            },
            SqlError::AuthenticationFailed { user, cause } => ConnectorError {
                user_facing_error: render_quaint_error(&cause, connection_info),
                kind: ErrorKind::AuthenticationFailed { user },
            },
            SqlError::ConnectTimeout(cause) => {
                let user_facing_error = render_quaint_error(&cause, connection_info);

                ConnectorError {
                    user_facing_error,
                    kind: ErrorKind::ConnectTimeout,
                }
            }
            SqlError::Timeout => ConnectorError::from_kind(ErrorKind::Timeout),
            SqlError::TlsError { cause } => {
                let user_facing_error = render_quaint_error(&cause, connection_info);
                ConnectorError {
                    user_facing_error,
                    kind: ErrorKind::TlsError {
                        message: format!("{}", cause),
                    },
                }
            }
            SqlError::ConnectionError { cause } => {
                let user_facing_error = render_quaint_error(&cause, connection_info);
                ConnectorError {
                    user_facing_error,
                    kind: ErrorKind::ConnectionError {
                        host: connection_info.host().to_owned(),
                        cause: cause.into(),
                    },
                }
            }
            SqlError::UniqueConstraintViolation { cause, .. } => {
                let user_facing_error = render_quaint_error(&cause, connection_info);
                ConnectorError {
                    user_facing_error,
                    kind: ErrorKind::ConnectionError {
                        host: connection_info.host().to_owned(),
                        cause: cause.into(),
                    },
                }
            }
            error => ConnectorError::from_kind(ErrorKind::QueryError(error.into())),
        }
    }
}

impl From<quaint::error::Error> for SqlError {
    fn from(error: quaint::error::Error) -> Self {
        match &error {
            quaint::error::Error::DatabaseDoesNotExist { db_name } => Self::DatabaseDoesNotExist {
                db_name: db_name.clone(),
                cause: error,
            },
            quaint::error::Error::DatabaseAlreadyExists { db_name } => Self::DatabaseAlreadyExists {
                db_name: db_name.clone(),
                cause: error,
            },
            quaint::error::Error::DatabaseAccessDenied { db_name } => Self::DatabaseAccessDenied {
                db_name: db_name.clone(),
                cause: error,
            },
            quaint::error::Error::AuthenticationFailed { user } => Self::AuthenticationFailed {
                user: user.clone(),
                cause: error,
            },
            quaint::error::Error::ConnectTimeout => Self::ConnectTimeout(error),
            quaint::error::Error::ConnectionError { .. } => Self::ConnectionError { cause: error },
            quaint::error::Error::Timeout => Self::Timeout,
            quaint::error::Error::TlsError { .. } => Self::TlsError { cause: error },
            quaint::error::Error::UniqueConstraintViolation { constraint } => match constraint {
                DatabaseConstraint::Fields(fields) => Self::UniqueConstraintViolation {
                    field_name: fields.join(","),
                    cause: error,
                },
                DatabaseConstraint::Index(index) => Self::UniqueConstraintViolation {
                    field_name: index.into(),
                    cause: error,
                }
            },
            _ => SqlError::QueryError(error.into()),
        }
    }
}

impl From<sql_schema_describer::SqlSchemaDescriberError> for SqlError {
    fn from(error: sql_schema_describer::SqlSchemaDescriberError) -> Self {
        SqlError::QueryError(anyhow::anyhow!("{}", error))
    }
}

impl From<String> for SqlError {
    fn from(error: String) -> Self {
        SqlError::Generic(anyhow::anyhow!(error))
    }
}
