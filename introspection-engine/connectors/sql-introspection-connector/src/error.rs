//! Introspection error handling.

use introspection_connector::{ConnectorError, ErrorKind};
use quaint::error::{Error as QuaintError, ErrorKind as QuaintKind};
use thiserror::Error;
use user_facing_errors::introspection_engine::DatabaseSchemaInconsistent;
use user_facing_errors::{common, quaint::render_quaint_error, query_engine::DatabaseConstraint, KnownError};

pub type SqlResult<T> = Result<T, SqlError>;

#[derive(Debug, Error)]
pub enum SqlError {
    #[error("{0}")]
    Generic(#[source] anyhow::Error),

    #[error("Error connecting to the database {cause}")]
    ConnectionError {
        #[source]
        cause: QuaintKind,
    },

    #[error("Error querying the database: {}", _0)]
    QueryError(#[source] anyhow::Error),

    #[error("{}", _0)]
    CrossSchemaReference(String),

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

    #[error("{}", _0)]
    DatabaseUrlIsInvalid(String),

    #[error("Connect timed out")]
    ConnectTimeout(#[source] QuaintKind),

    #[error("Operation timed out ({0})")]
    Timeout(String),

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

    #[error("An Error occurred because the schema was inconsistent: '{}'", explanation)]
    SchemaInconsistent { explanation: String },
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
            SqlError::Timeout(message) => ConnectorError::from_kind(ErrorKind::Timeout(message)),
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
            SqlError::SchemaInconsistent { explanation } => ConnectorError {
                user_facing_error: Some(KnownError::new(DatabaseSchemaInconsistent {
                    explanation: explanation.to_owned(),
                })),
                kind: ErrorKind::DatabaseSchemaInconsistent { explanation },
            },
            SqlError::DatabaseUrlIsInvalid(reason) => {
                let user_facing_error = Some(KnownError::new(common::InvalidConnectionString {
                    details: reason.clone(),
                }));

                ConnectorError {
                    user_facing_error,
                    kind: ErrorKind::InvalidDatabaseUrl(reason),
                }
            }
            SqlError::CrossSchemaReference(explanation) => ConnectorError {
                user_facing_error: Some(KnownError::new(DatabaseSchemaInconsistent {
                    explanation: explanation.clone(),
                })),
                kind: ErrorKind::DatabaseSchemaInconsistent { explanation },
            },
            error => ConnectorError::from_kind(ErrorKind::QueryError(error.into())),
        }
    }
}

impl From<QuaintKind> for SqlError {
    fn from(kind: QuaintKind) -> Self {
        match kind {
            QuaintKind::DatabaseDoesNotExist { ref db_name } => Self::DatabaseDoesNotExist {
                db_name: format!("{}", db_name),
                cause: kind,
            },
            QuaintKind::DatabaseAlreadyExists { ref db_name } => Self::DatabaseAlreadyExists {
                db_name: format!("{}", db_name),
                cause: kind,
            },
            QuaintKind::DatabaseAccessDenied { ref db_name } => Self::DatabaseAccessDenied {
                db_name: format!("{}", db_name),
                cause: kind,
            },
            QuaintKind::AuthenticationFailed { ref user } => Self::AuthenticationFailed {
                user: format!("{}", user),
                cause: kind,
            },
            QuaintKind::DatabaseUrlIsInvalid(reason) => Self::DatabaseUrlIsInvalid(reason),
            e @ QuaintKind::ConnectTimeout => Self::ConnectTimeout(e),
            e @ QuaintKind::SocketTimeout => Self::Timeout(format!("{}", e)),
            e @ QuaintKind::PoolTimeout { .. } => Self::Timeout(format!("{}", e)),
            QuaintKind::ConnectionError { .. } => Self::ConnectionError { cause: kind },
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

impl From<sql_schema_describer::DescriberError> for SqlError {
    fn from(error: sql_schema_describer::DescriberError) -> Self {
        match error.kind() {
            sql_schema_describer::DescriberErrorKind::QuaintError(..) => {
                SqlError::QueryError(anyhow::anyhow!("{}", error))
            }
            sql_schema_describer::DescriberErrorKind::CrossSchemaReference { .. } => {
                SqlError::CrossSchemaReference(format!("{}", error))
            }
        }
    }
}

impl From<String> for SqlError {
    fn from(error: String) -> Self {
        SqlError::Generic(anyhow::anyhow!(error))
    }
}
