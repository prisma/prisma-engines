use connector_interface::error::*;
use quaint::error::ErrorKind as QuaintKind;
use query_structure::{prelude::DomainError, Filter};
use std::{any::Any, string::FromUtf8Error};
use thiserror::Error;
use user_facing_errors::query_engine::DatabaseConstraint;

#[cfg(not(target_arch = "wasm32"))]
use quaint::error::NativeErrorKind;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) enum NativeRawError {
    ConnectionClosed,
}

pub(crate) enum RawError {
    #[cfg(not(target_arch = "wasm32"))]
    Native(NativeRawError),
    Database {
        code: Option<String>,
        message: Option<String>,
    },
    UnsupportedColumnType {
        column_type: String,
    },
    IncorrectNumberOfParameters {
        expected: usize,
        actual: usize,
    },
    QueryInvalidInput(String),
    External {
        id: i32,
    },
    ConversionError(anyhow::Error),
}

impl From<RawError> for SqlError {
    fn from(re: RawError) -> SqlError {
        match re {
            #[cfg(not(target_arch = "wasm32"))]
            RawError::Native(native) => match native {
                NativeRawError::ConnectionClosed => SqlError::ConnectionClosed,
            },
            RawError::IncorrectNumberOfParameters { expected, actual } => {
                Self::IncorrectNumberOfParameters { expected, actual }
            }
            RawError::QueryInvalidInput(message) => Self::QueryInvalidInput(message),
            RawError::UnsupportedColumnType { column_type } => Self::RawError {
                code: String::from("N/A"),
                message: format!(
                    r#"Failed to deserialize column of type '{column_type}'. If you're using $queryRaw and this column is explicitly marked as `Unsupported` in your Prisma schema, try casting this column to any supported Prisma type such as `String`."#
                ),
            },
            RawError::Database { code, message } => Self::RawError {
                code: code.unwrap_or_else(|| String::from("N/A")),
                message: message.unwrap_or_else(|| String::from("N/A")),
            },
            RawError::External { id } => Self::ExternalError(id),
            RawError::ConversionError(err) => Self::ConversionError(err),
        }
    }
}

impl From<quaint::error::Error> for RawError {
    fn from(e: quaint::error::Error) -> Self {
        let default_value: RawError = Self::Database {
            code: e.original_code().map(ToString::to_string),
            message: e.original_message().map(ToString::to_string),
        };

        match e.kind() {
            #[cfg(not(target_arch = "wasm32"))]
            quaint::error::ErrorKind::Native(NativeErrorKind::ConnectionClosed) => {
                Self::Native(NativeRawError::ConnectionClosed)
            }
            quaint::error::ErrorKind::IncorrectNumberOfParameters { expected, actual } => {
                Self::IncorrectNumberOfParameters {
                    expected: *expected,
                    actual: *actual,
                }
            }
            quaint::error::ErrorKind::UnsupportedColumnType { column_type } => Self::UnsupportedColumnType {
                column_type: column_type.to_owned(),
            },
            quaint::error::ErrorKind::QueryInvalidInput(message) => Self::QueryInvalidInput(message.to_owned()),
            quaint::error::ErrorKind::ExternalError(id) => Self::External { id: *id },
            _ => default_value,
        }
    }
}

impl From<serde_json::error::Error> for RawError {
    fn from(e: serde_json::error::Error) -> Self {
        Self::ConversionError(e.into())
    }
}

// Catching the panics from the database driver for better error messages.
impl From<Box<dyn Any + Send>> for RawError {
    fn from(e: Box<dyn Any + Send>) -> Self {
        Self::Database {
            code: None,
            message: Some(*e.downcast::<String>().unwrap()),
        }
    }
}

#[derive(Debug, Error)]
pub enum SqlError {
    #[error("Unique constraint failed: {:?}", constraint)]
    UniqueConstraintViolation { constraint: DatabaseConstraint },

    #[error("Null constraint failed: {:?}", constraint)]
    NullConstraintViolation { constraint: DatabaseConstraint },

    #[error("Foreign key constraint failed")]
    ForeignKeyConstraintViolation { constraint: DatabaseConstraint },

    #[error("Record does not exist: {cause}")]
    RecordDoesNotExist { cause: String },

    #[error("Table {} does not exist", _0)]
    TableDoesNotExist(String),

    #[error("Column {} does not exist", _0)]
    ColumnDoesNotExist(String),

    #[error("Error creating a database connection. ({})", _0)]
    ConnectionError(QuaintKind),

    #[error("Error querying the database: {}", _0)]
    QueryError(Box<dyn std::error::Error + Send + Sync>),

    #[error("Invalid input provided to query: {}", _0)]
    QueryInvalidInput(String),

    #[error("The column value was different from the model")]
    ColumnReadFailure(Box<dyn std::error::Error + Send + Sync>),

    #[error("Field cannot be null: {}", field)]
    FieldCannotBeNull { field: String },

    #[error("{}", _0)]
    DomainError(DomainError),

    #[error("Record not found: {:?}", _0)]
    RecordNotFoundForWhere(Filter),

    #[error(
        "Violating a relation {} between {} and {}",
        relation_name,
        model_a_name,
        model_b_name
    )]
    RelationViolation {
        relation_name: String,
        model_a_name: String,
        model_b_name: String,
    },

    #[error(
        "The relation {} has no record for the model {} connected to a record for the model {} on your write path.",
        relation_name,
        parent_name,
        child_name
    )]
    RecordsNotConnected {
        relation_name: String,
        parent_name: String,
        // parent_where: Option<Box<RecordFinderInfo>>,
        child_name: String,
        // child_where: Option<Box<RecordFinderInfo>>,
    },

    #[error("Conversion error: {0}")]
    ConversionError(anyhow::Error),

    #[error("Database error. error code: {}, error message: {}", code, message)]
    RawError { code: String, message: String },

    #[error(
        "Incorrect number of parameters given to a statement. Expected {}: got: {}.",
        expected,
        actual
    )]
    IncorrectNumberOfParameters { expected: usize, actual: usize },

    #[error("Server terminated the connection.")]
    ConnectionClosed,

    #[error("{}", _0)]
    TransactionAlreadyClosed(String),

    #[error("{}", _0)]
    InvalidIsolationLevel(String),

    #[error("Transaction write conflict")]
    TransactionWriteConflict,

    #[error("ROLLBACK statement has no corresponding BEGIN statement")]
    RollbackWithoutBegin,

    #[error("Query parameter limit exceeded error: {0}.")]
    QueryParameterLimitExceeded(String),

    #[error("Cannot find a fulltext index to use for the search")]
    MissingFullTextSearchIndex,

    #[error("External connector error")]
    ExternalError(i32),

    #[error("Too many DB connections opened")]
    TooManyConnections(Box<dyn std::error::Error + Send + Sync>),
}

impl SqlError {
    pub(crate) fn into_connector_error(self, connection_info: &quaint::prelude::ConnectionInfo) -> ConnectorError {
        match self {
            SqlError::UniqueConstraintViolation { constraint } => {
                ConnectorError::from_kind(ErrorKind::UniqueConstraintViolation { constraint })
            }
            SqlError::NullConstraintViolation { constraint } => {
                ConnectorError::from_kind(ErrorKind::NullConstraintViolation { constraint })
            }
            SqlError::ForeignKeyConstraintViolation { constraint } => {
                ConnectorError::from_kind(ErrorKind::ForeignKeyConstraintViolation { constraint })
            }
            SqlError::RecordDoesNotExist { cause } => {
                ConnectorError::from_kind(ErrorKind::RecordDoesNotExist { cause })
            }
            SqlError::TableDoesNotExist(table) => ConnectorError::from_kind(ErrorKind::TableDoesNotExist { table }),
            SqlError::ColumnDoesNotExist(column) => ConnectorError::from_kind(ErrorKind::ColumnDoesNotExist { column }),
            SqlError::ConnectionError(e) => ConnectorError {
                user_facing_error: user_facing_errors::quaint::render_quaint_error(&e, connection_info),
                kind: ErrorKind::ConnectionError(e.into()),
                transient: false,
            },
            SqlError::ColumnReadFailure(e) => ConnectorError::from_kind(ErrorKind::ColumnReadFailure(e)),
            SqlError::FieldCannotBeNull { field } => ConnectorError::from_kind(ErrorKind::FieldCannotBeNull { field }),
            SqlError::DomainError(e) => ConnectorError::from_kind(ErrorKind::DomainError(e)),
            SqlError::RecordNotFoundForWhere(info) => {
                ConnectorError::from_kind(ErrorKind::RecordNotFoundForWhere(info))
            }
            SqlError::RelationViolation {
                relation_name,
                model_a_name,
                model_b_name,
            } => ConnectorError::from_kind(ErrorKind::RelationViolation {
                relation_name,
                model_a_name,
                model_b_name,
            }),
            SqlError::RecordsNotConnected {
                relation_name,
                parent_name,
                child_name,
            } => ConnectorError::from_kind(ErrorKind::RecordsNotConnected {
                relation_name,
                parent_name,
                child_name,
            }),
            SqlError::ConversionError(e) => ConnectorError::from_kind(ErrorKind::ConversionError(e)),
            SqlError::QueryInvalidInput(e) => ConnectorError::from_kind(ErrorKind::QueryInvalidInput(e)),
            SqlError::IncorrectNumberOfParameters { expected, actual } => {
                ConnectorError::from_kind(ErrorKind::IncorrectNumberOfParameters { expected, actual })
            }
            SqlError::QueryError(e) => {
                let quaint_error: Option<&QuaintKind> = e.downcast_ref();
                match quaint_error {
                    Some(quaint_error) => ConnectorError {
                        user_facing_error: user_facing_errors::quaint::render_quaint_error(
                            quaint_error,
                            connection_info,
                        ),
                        kind: ErrorKind::QueryError(e),
                        transient: false,
                    },
                    None => ConnectorError::from_kind(ErrorKind::QueryError(e)),
                }
            }
            SqlError::RawError { code, message } => {
                ConnectorError::from_kind(ErrorKind::RawDatabaseError { code, message })
            }
            SqlError::ConnectionClosed => ConnectorError::from_kind(ErrorKind::ConnectionClosed),
            SqlError::TransactionAlreadyClosed(message) => {
                ConnectorError::from_kind(ErrorKind::TransactionAlreadyClosed { message })
            }
            SqlError::TransactionWriteConflict => ConnectorError::from_kind(ErrorKind::TransactionWriteConflict),
            SqlError::RollbackWithoutBegin => ConnectorError::from_kind(ErrorKind::RollbackWithoutBegin),
            SqlError::QueryParameterLimitExceeded(e) => {
                ConnectorError::from_kind(ErrorKind::QueryParameterLimitExceeded(e))
            }
            SqlError::MissingFullTextSearchIndex => {
                ConnectorError::from_kind(ErrorKind::NativeMissingFullTextSearchIndex)
            }
            SqlError::InvalidIsolationLevel(msg) => ConnectorError::from_kind(ErrorKind::InternalConversionError(msg)),
            SqlError::ExternalError(error_id) => ConnectorError::from_kind(ErrorKind::ExternalError(error_id)),
            SqlError::TooManyConnections(e) => ConnectorError::from_kind(ErrorKind::TooManyConnections(e)),
        }
    }
}

impl From<query_structure::ConversionFailure> for SqlError {
    fn from(e: query_structure::ConversionFailure) -> Self {
        Self::ConversionError(e.into())
    }
}

impl From<quaint::error::Error> for SqlError {
    fn from(error: quaint::error::Error) -> Self {
        let quaint_kind = QuaintKind::from(error);

        match quaint_kind {
            #[cfg(not(target_arch = "wasm32"))]
            QuaintKind::Native(ref native_error_kind) => match native_error_kind {
                NativeErrorKind::IoError(_) | NativeErrorKind::ConnectionError(_) => Self::ConnectionError(quaint_kind),
                NativeErrorKind::ConnectionClosed => SqlError::ConnectionClosed,
                NativeErrorKind::ConnectTimeout => SqlError::ConnectionError(quaint_kind),
                NativeErrorKind::PoolTimeout { .. } => SqlError::ConnectionError(quaint_kind),
                NativeErrorKind::PoolClosed { .. } => SqlError::ConnectionError(quaint_kind),
                NativeErrorKind::TlsError { .. } => Self::ConnectionError(quaint_kind),
            },

            QuaintKind::RawConnectorError { status, reason } => Self::RawError {
                code: status,
                message: reason,
            },
            QuaintKind::QueryError(qe) => Self::QueryError(qe),
            QuaintKind::QueryInvalidInput(qe) => Self::QueryInvalidInput(qe),
            QuaintKind::NotFound => Self::RecordDoesNotExist {
                cause: "Record not found".to_owned(),
            },
            QuaintKind::UniqueConstraintViolation { constraint } => Self::UniqueConstraintViolation {
                constraint: constraint.into(),
            },

            QuaintKind::NullConstraintViolation { constraint } => Self::NullConstraintViolation {
                constraint: constraint.into(),
            },

            QuaintKind::ForeignKeyConstraintViolation { constraint } => Self::ForeignKeyConstraintViolation {
                constraint: constraint.into(),
            },
            QuaintKind::MissingFullTextSearchIndex => Self::MissingFullTextSearchIndex,
            QuaintKind::ColumnReadFailure(e) => Self::ColumnReadFailure(e),
            QuaintKind::ColumnNotFound { column } => SqlError::ColumnDoesNotExist(format!("{column}")),
            QuaintKind::TableDoesNotExist { table } => SqlError::TableDoesNotExist(format!("{table}")),

            QuaintKind::InvalidIsolationLevel(msg) => Self::InvalidIsolationLevel(msg),
            QuaintKind::TransactionWriteConflict => Self::TransactionWriteConflict,
            QuaintKind::RollbackWithoutBegin => Self::RollbackWithoutBegin,
            QuaintKind::ExternalError(error_id) => Self::ExternalError(error_id),
            QuaintKind::TooManyConnections(e) => Self::TooManyConnections(e),
            e @ QuaintKind::UnsupportedColumnType { .. } => SqlError::ConversionError(e.into()),
            e @ QuaintKind::TransactionAlreadyClosed(_) => SqlError::TransactionAlreadyClosed(format!("{e}")),
            e @ QuaintKind::IncorrectNumberOfParameters { .. } => SqlError::QueryError(e.into()),
            e @ QuaintKind::ConversionError(_) => SqlError::ConversionError(e.into()),
            e @ QuaintKind::ResultIndexOutOfBounds { .. } => SqlError::QueryError(e.into()),
            e @ QuaintKind::ResultTypeMismatch { .. } => SqlError::QueryError(e.into()),
            e @ QuaintKind::LengthMismatch { .. } => SqlError::QueryError(e.into()),
            e @ QuaintKind::ValueOutOfRange { .. } => SqlError::QueryError(e.into()),
            e @ QuaintKind::UUIDError(_) => SqlError::ConversionError(e.into()),
            e @ QuaintKind::DatabaseUrlIsInvalid { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::DatabaseDoesNotExist { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::AuthenticationFailed { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::DatabaseAccessDenied { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::DatabaseAlreadyExists { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::InvalidConnectionArguments => SqlError::ConnectionError(e),
            e @ QuaintKind::SocketTimeout => SqlError::ConnectionError(e),
        }
    }
}

impl From<DomainError> for SqlError {
    fn from(e: DomainError) -> SqlError {
        SqlError::DomainError(e)
    }
}

impl From<serde_json::error::Error> for SqlError {
    fn from(e: serde_json::error::Error) -> SqlError {
        SqlError::ConversionError(e.into())
    }
}

impl From<uuid::Error> for SqlError {
    fn from(e: uuid::Error) -> SqlError {
        SqlError::ColumnReadFailure(e.into())
    }
}

impl From<FromUtf8Error> for SqlError {
    fn from(e: FromUtf8Error) -> SqlError {
        SqlError::ColumnReadFailure(e.into())
    }
}
