use connector_interface::{error::*, Filter};
use failure::{Error, Fail};
use prisma_models::prelude::DomainError;
use quaint::error::ErrorKind as QuaintKind;
use std::{any::Any, string::FromUtf8Error};
use user_facing_errors::query_engine::DatabaseConstraint;

pub struct RawError {
    code: Option<String>,
    message: Option<String>,
}

impl From<RawError> for SqlError {
    fn from(re: RawError) -> SqlError {
        Self::RawError {
            code: re.code.unwrap_or_else(|| String::from("N/A")),
            message: re.message.unwrap_or_else(|| String::from("N/A")),
        }
    }
}

impl From<quaint::error::Error> for RawError {
    fn from(e: quaint::error::Error) -> Self {
        Self {
            code: e.original_code().map(ToString::to_string),
            message: e.original_message().map(ToString::to_string),
        }
    }
}

// Catching the panics from the database driver for better error messages.
impl From<Box<dyn Any + Send>> for RawError {
    fn from(e: Box<dyn Any + Send>) -> Self {
        Self {
            code: None,
            message: Some(*e.downcast::<String>().unwrap()),
        }
    }
}

#[derive(Debug, Fail)]
pub enum SqlError {
    #[fail(display = "Unique constraint failed: {:?}", constraint)]
    UniqueConstraintViolation { constraint: DatabaseConstraint },

    #[fail(display = "Null constraint failed: {:?}", constraint)]
    NullConstraintViolation { constraint: DatabaseConstraint },

    #[fail(display = "Record does not exist.")]
    RecordDoesNotExist,

    #[fail(display = "Column does not exist")]
    ColumnDoesNotExist,

    #[fail(display = "Error creating a database connection.")]
    ConnectionError(QuaintKind),

    #[fail(display = "Error querying the database: {}", _0)]
    QueryError(Box<dyn std::error::Error + Send + Sync>),

    #[fail(display = "The column value was different from the model")]
    ColumnReadFailure(Box<dyn std::error::Error + Send + Sync>),

    #[fail(display = "Field cannot be null: {}", field)]
    FieldCannotBeNull { field: String },

    #[fail(display = "{}", _0)]
    DomainError(DomainError),

    #[fail(display = "Record not found: {:?}", _0)]
    RecordNotFoundForWhere(Filter),

    #[fail(
        display = "Violating a relation {} between {} and {}",
        relation_name, model_a_name, model_b_name
    )]
    RelationViolation {
        relation_name: String,
        model_a_name: String,
        model_b_name: String,
    },

    #[fail(
        display = "The relation {} has no record for the model {} connected to a record for the model {} on your write path.",
        relation_name, parent_name, child_name
    )]
    RecordsNotConnected {
        relation_name: String,
        parent_name: String,
        // parent_where: Option<Box<RecordFinderInfo>>,
        child_name: String,
        // child_where: Option<Box<RecordFinderInfo>>,
    },

    #[fail(display = "Conversion error: {}", _0)]
    ConversionError(Error),

    #[fail(display = "Database error. error code: {}, error message: {}", code, message)]
    RawError { code: String, message: String },
}

impl SqlError {
    pub(crate) fn into_connector_error(self, connection_info: &quaint::prelude::ConnectionInfo) -> ConnectorError {
        match self {
            SqlError::UniqueConstraintViolation { constraint } => ConnectorError {
                user_facing_error: user_facing_errors::KnownError::new(
                    user_facing_errors::query_engine::UniqueKeyViolation {
                        constraint: constraint.clone(),
                    },
                )
                .ok(),
                kind: ErrorKind::UniqueConstraintViolation { constraint },
            },
            SqlError::NullConstraintViolation { constraint } => {
                ConnectorError::from_kind(ErrorKind::NullConstraintViolation { constraint })
            }
            SqlError::RecordDoesNotExist => ConnectorError::from_kind(ErrorKind::RecordDoesNotExist),
            SqlError::ColumnDoesNotExist => ConnectorError::from_kind(ErrorKind::ColumnDoesNotExist),
            SqlError::ConnectionError(e) => ConnectorError {
                user_facing_error: user_facing_errors::quaint::render_quaint_error(&e, connection_info),
                kind: ErrorKind::ConnectionError(e.into()),
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
            SqlError::QueryError(e) => ConnectorError::from_kind(ErrorKind::QueryError(e)),
            SqlError::RawError { code, message } => ConnectorError {
                user_facing_error: user_facing_errors::KnownError::new(
                    user_facing_errors::query_engine::RawQueryFailed {
                        code: code.clone(),
                        message: message.clone(),
                    },
                )
                .ok(),
                kind: ErrorKind::RawError { code, message },
            },
        }
    }
}

impl From<quaint::error::Error> for SqlError {
    fn from(e: quaint::error::Error) -> Self {
        match QuaintKind::from(e) {
            QuaintKind::QueryError(qe) => Self::QueryError(qe),
            e @ QuaintKind::IoError(_) => Self::ConnectionError(e),
            QuaintKind::NotFound => Self::RecordDoesNotExist,
            QuaintKind::UniqueConstraintViolation { constraint } => Self::UniqueConstraintViolation {
                constraint: constraint.into(),
            },

            QuaintKind::NullConstraintViolation { constraint } => Self::NullConstraintViolation {
                constraint: constraint.into(),
            },

            e @ QuaintKind::ConnectionError(_) => Self::ConnectionError(e),
            QuaintKind::ColumnReadFailure(e) => Self::ColumnReadFailure(e),
            QuaintKind::ColumnNotFound(_) => Self::ColumnDoesNotExist,
            e @ QuaintKind::ConversionError(_) => SqlError::ConversionError(e.into()),
            e @ QuaintKind::ResultIndexOutOfBounds { .. } => SqlError::QueryError(e.into()),
            e @ QuaintKind::ResultTypeMismatch { .. } => SqlError::QueryError(e.into()),
            e @ QuaintKind::DatabaseUrlIsInvalid { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::DatabaseDoesNotExist { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::AuthenticationFailed { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::DatabaseAccessDenied { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::DatabaseAlreadyExists { .. } => SqlError::ConnectionError(e),
            e @ QuaintKind::InvalidConnectionArguments => SqlError::ConnectionError(e),
            e @ QuaintKind::ConnectTimeout { .. } => SqlError::ConnectionError(e.into()),
            e @ QuaintKind::Timeout(..) => SqlError::ConnectionError(e.into()),
            e @ QuaintKind::TlsError { .. } => Self::ConnectionError(e.into()),
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
