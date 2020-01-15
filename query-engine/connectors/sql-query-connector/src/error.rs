use connector_interface::{error::*, Filter};
use failure::{Error, Fail};
use prisma_models::prelude::DomainError;
use quaint::error::Error as QuaintError;
use std::string::FromUtf8Error;

#[derive(Debug, Fail)]
pub enum SqlError {
    #[fail(display = "Unique constraint failed: {:?}", field_names)]
    UniqueConstraintViolation { field_names: Vec<String> },

    #[fail(display = "Null constraint failed: {}", field_name)]
    NullConstraintViolation { field_name: String },

    #[fail(display = "Record does not exist.")]
    RecordDoesNotExist,

    #[fail(display = "Column does not exist")]
    ColumnDoesNotExist,

    #[fail(display = "Error creating a database connection.")]
    ConnectionError(QuaintError),

    #[fail(display = "Error querying the database: {}", _0)]
    QueryError(Box<dyn std::error::Error + Send + Sync>),

    #[fail(display = "The column value was different from the model")]
    ColumnReadFailure(Box<dyn std::error::Error + Send + Sync>),

    #[fail(display = "Field cannot be null: {}", field)]
    FieldCannotBeNull { field: String },

    #[fail(display = "{}", _0)]
    DomainError(DomainError),

    #[fail(display = "Record not found: {}", _0)]
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
}

impl SqlError {
    pub(crate) fn into_connector_error(self, connection_info: &quaint::prelude::ConnectionInfo) -> ConnectorError {
        match self {
            SqlError::UniqueConstraintViolation { field_names } => ConnectorError {
                user_facing_error: user_facing_errors::KnownError::new(
                    user_facing_errors::query_engine::UniqueKeyViolation {
                        constraint: user_facing_errors::query_engine::DatabaseConstraint::Fields(field_names.clone())
                    },
                )
                .ok(),
                kind: ErrorKind::UniqueConstraintViolation { field_name: field_names.join(", ") },
            },
            SqlError::NullConstraintViolation { field_name } => {
                ConnectorError::from_kind(ErrorKind::NullConstraintViolation { field_name })
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
        }
    }
}

impl From<quaint::error::Error> for SqlError {
    fn from(e: quaint::error::Error) -> Self {
        match e {
            quaint::error::Error::QueryError(e) => Self::QueryError(e),
            quaint::error::Error::IoError(_) => Self::ConnectionError(e),
            quaint::error::Error::NotFound => Self::RecordDoesNotExist,
            quaint::error::Error::UniqueConstraintViolation { constraint } => {
                match constraint {
                    quaint::error::DatabaseConstraint::Fields(field_names) => {
                        Self::UniqueConstraintViolation { field_names }
                    },
                    quaint::error::DatabaseConstraint::Index(_) => {
                        Self::UniqueConstraintViolation { field_names: vec![] }
                    }
                }
            }

            quaint::error::Error::NullConstraintViolation { constraint } => {
                match constraint {
                    quaint::error::DatabaseConstraint::Fields(field_names) => {
                        Self::NullConstraintViolation { field_name: field_names.join(", ") }
                    },
                    quaint::error::DatabaseConstraint::Index(index) => {
                        Self::NullConstraintViolation { field_name: index }
                    }
                }
            }

            quaint::error::Error::ConnectionError(_) => Self::ConnectionError(e),
            quaint::error::Error::ColumnReadFailure(e) => Self::ColumnReadFailure(e),
            quaint::error::Error::ColumnNotFound(_) => Self::ColumnDoesNotExist,

            e @ quaint::error::Error::ConversionError(_) => SqlError::ConversionError(e.into()),
            e @ quaint::error::Error::ResultIndexOutOfBounds { .. } => SqlError::QueryError(e.into()),
            e @ quaint::error::Error::ResultTypeMismatch { .. } => SqlError::QueryError(e.into()),
            e @ quaint::error::Error::DatabaseUrlIsInvalid { .. } => SqlError::ConnectionError(e),
            e @ quaint::error::Error::DatabaseDoesNotExist { .. } => SqlError::ConnectionError(e),
            e @ quaint::error::Error::AuthenticationFailed { .. } => SqlError::ConnectionError(e),
            e @ quaint::error::Error::DatabaseAccessDenied { .. } => SqlError::ConnectionError(e),
            e @ quaint::error::Error::DatabaseAlreadyExists { .. } => SqlError::ConnectionError(e),
            e @ quaint::error::Error::InvalidConnectionArguments => SqlError::ConnectionError(e),
            e @ quaint::error::Error::ConnectTimeout { .. } => SqlError::ConnectionError(e.into()),
            e @ quaint::error::Error::Timeout => SqlError::ConnectionError(e.into()),
            e @ quaint::error::Error::TlsError { .. } => Self::ConnectionError(e.into()),
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

impl From<url::ParseError> for SqlError {
    fn from(err: url::ParseError) -> SqlError {
        let quaint_error = QuaintError::from(err);
        SqlError::from(quaint_error)
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
