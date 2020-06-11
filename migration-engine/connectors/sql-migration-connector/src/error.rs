use migration_connector::{ConnectorError, ErrorKind};
use quaint::error::{Error as QuaintError, ErrorKind as QuaintKind};
use thiserror::Error;
use tracing_error::SpanTrace;
use user_facing_errors::{
    migration_engine::MigrateSystemDatabase, quaint::render_quaint_error, query_engine::DatabaseConstraint, KnownError,
};

pub type SqlResult<T> = Result<T, SqlError>;

#[derive(Debug, Error)]
pub enum SqlError {
    #[error(transparent)]
    Generic(anyhow::Error),

    #[error("Error connecting to the database {cause}")]
    ConnectionError {
        #[source]
        cause: QuaintKind,
    },

    #[error(transparent)]
    QueryError(anyhow::Error),

    #[error("Database '{}' does not exist", db_name)]
    DatabaseDoesNotExist {
        db_name: String,
        #[source]
        cause: QuaintKind,
    },

    #[error("Access denied to database '{}'", db_name)]
    DatabaseAccessDenied {
        db_name: String,
        #[source]
        cause: QuaintKind,
    },

    #[error("Database '{}' already exists", db_name)]
    DatabaseAlreadyExists {
        db_name: String,
        #[source]
        cause: QuaintKind,
    },

    #[error("Authentication failed for user '{}'", user)]
    AuthenticationFailed {
        user: String,
        #[source]
        cause: QuaintKind,
    },

    #[error("Connect timed out")]
    ConnectTimeout(#[source] QuaintKind),

    #[error("Operation timed out")]
    Timeout,

    #[error("Error opening a TLS connection. {}", cause)]
    TlsError {
        #[source]
        cause: QuaintKind,
    },

    #[error("Unique constraint violation")]
    UniqueConstraintViolation {
        constraint: DatabaseConstraint,
        #[source]
        cause: QuaintKind,
    },
}

impl SqlError {
    pub(crate) fn into_connector_error(self, connection_info: &super::ConnectionInfo) -> ConnectorError {
        let context = SpanTrace::capture();

        match self {
            SqlError::DatabaseDoesNotExist { db_name, cause } => ConnectorError {
                user_facing_error: render_quaint_error(&cause, connection_info),
                kind: ErrorKind::DatabaseDoesNotExist { db_name },
                context,
            },
            SqlError::DatabaseAccessDenied { db_name, cause } => ConnectorError {
                user_facing_error: render_quaint_error(&cause, connection_info),
                kind: ErrorKind::DatabaseAccessDenied { database_name: db_name },
                context,
            },

            SqlError::DatabaseAlreadyExists { db_name, cause } => ConnectorError {
                user_facing_error: render_quaint_error(&cause, connection_info),
                kind: ErrorKind::DatabaseAlreadyExists { db_name },
                context,
            },
            SqlError::AuthenticationFailed { user, cause } => ConnectorError {
                user_facing_error: render_quaint_error(&cause, connection_info),
                kind: ErrorKind::AuthenticationFailed { user },
                context,
            },
            SqlError::ConnectTimeout(cause) => {
                let user_facing_error = render_quaint_error(&cause, connection_info);

                ConnectorError {
                    user_facing_error,
                    kind: ErrorKind::ConnectTimeout,
                    context,
                }
            }
            SqlError::Timeout => ConnectorError::from_kind(ErrorKind::Timeout),
            SqlError::TlsError { cause } => {
                let user_facing_error = render_quaint_error(&cause, connection_info);

                ConnectorError {
                    user_facing_error,
                    kind: ErrorKind::TlsError {
                        message: cause.to_string(),
                    },
                    context,
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
                    context,
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
                    context,
                }
            }
            error => ConnectorError::from_kind(ErrorKind::QueryError(error.into())),
        }
    }
}

impl From<QuaintKind> for SqlError {
    fn from(kind: QuaintKind) -> Self {
        match kind {
            QuaintKind::DatabaseDoesNotExist { ref db_name } => Self::DatabaseDoesNotExist {
                db_name: db_name.clone(),
                cause: kind,
            },
            QuaintKind::DatabaseAlreadyExists { ref db_name } => Self::DatabaseAlreadyExists {
                db_name: db_name.clone(),
                cause: kind,
            },
            QuaintKind::DatabaseAccessDenied { ref db_name } => Self::DatabaseAccessDenied {
                db_name: db_name.clone(),
                cause: kind,
            },
            QuaintKind::AuthenticationFailed { ref user } => Self::AuthenticationFailed {
                user: user.clone(),
                cause: kind,
            },
            e @ QuaintKind::ConnectTimeout(..) => Self::ConnectTimeout(e),
            QuaintKind::ConnectionError { .. } => Self::ConnectionError { cause: kind },
            QuaintKind::Timeout(..) => Self::Timeout,
            QuaintKind::TlsError { .. } => Self::TlsError { cause: kind },
            QuaintKind::UniqueConstraintViolation { ref constraint } => Self::UniqueConstraintViolation {
                constraint: constraint.into(),
                cause: kind,
            },
            _ => SqlError::QueryError(kind.into()),
        }
    }
}

impl From<QuaintError> for SqlError {
    fn from(e: QuaintError) -> Self {
        QuaintKind::from(e).into()
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

pub(crate) type CheckDatabaseInfoResult = Result<(), SystemDatabase>;

#[derive(Debug, Error)]
#[error("The `{0}` database is a system database, it should not be altered with prisma migrate. Please connect to another database.")]
pub(crate) struct SystemDatabase(pub(crate) String);

impl From<SystemDatabase> for ConnectorError {
    fn from(err: SystemDatabase) -> ConnectorError {
        let user_facing = MigrateSystemDatabase {
            database_name: err.0.clone(),
        };

        ConnectorError {
            user_facing_error: Some(KnownError::new(user_facing).unwrap()),
            kind: ErrorKind::Generic(err.into()),
            context: SpanTrace::capture(),
        }
    }
}
