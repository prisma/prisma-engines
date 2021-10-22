use std::fmt::Display;

use crate::filter::Filter;
use itertools::Itertools;
use prisma_models::prelude::DomainError;
use thiserror::Error;
use user_facing_errors::{query_engine::DatabaseConstraint, KnownError};

#[derive(Debug, Error)]
#[error("{}", kind)]
pub struct ConnectorError {
    /// An optional error already rendered for users in case the migration core does not handle it.
    pub user_facing_error: Option<KnownError>,
    /// The error information for internal use.
    pub kind: ErrorKind,
}

impl ConnectorError {
    pub fn from_kind(kind: ErrorKind) -> Self {
        let user_facing_error = match &kind {
            ErrorKind::NullConstraintViolation { constraint } => Some(KnownError::new(
                user_facing_errors::query_engine::NullConstraintViolation {
                    constraint: constraint.to_owned(),
                },
            )),
            ErrorKind::TableDoesNotExist { table } => {
                Some(KnownError::new(user_facing_errors::query_engine::TableDoesNotExist {
                    table: table.clone(),
                }))
            }
            ErrorKind::ColumnDoesNotExist { column } => {
                Some(KnownError::new(user_facing_errors::query_engine::ColumnDoesNotExist {
                    column: column.clone(),
                }))
            }
            ErrorKind::InvalidDatabaseUrl { details, url: _ } => {
                let details = user_facing_errors::quaint::invalid_connection_string_description(details);

                Some(KnownError::new(user_facing_errors::common::InvalidConnectionString {
                    details,
                }))
            }
            ErrorKind::ForeignKeyConstraintViolation { constraint } => {
                let field_name = match constraint {
                    DatabaseConstraint::Fields(fields) => fields.join(","),
                    DatabaseConstraint::Index(index) => format!("{} (index)", index),
                    DatabaseConstraint::ForeignKey => "foreign key".to_string(),
                    DatabaseConstraint::CannotParse => "(not available)".to_string(),
                };

                Some(KnownError::new(user_facing_errors::query_engine::ForeignKeyViolation {
                    field_name,
                }))
            }
            ErrorKind::ConversionError(message) => Some(KnownError::new(
                user_facing_errors::query_engine::InconsistentColumnData {
                    message: format!("{}", message),
                },
            )),
            ErrorKind::UnsupportedFeature(feature) => {
                Some(KnownError::new(user_facing_errors::query_engine::UnsupportedFeature {
                    feature: feature.clone(),
                }))
            }
            ErrorKind::MultiError(merror) => Some(KnownError::new(user_facing_errors::query_engine::MultiError {
                errors: format!("{}", merror),
            })),
            ErrorKind::UniqueConstraintViolation { constraint } => {
                Some(KnownError::new(user_facing_errors::query_engine::UniqueKeyViolation {
                    constraint: constraint.clone(),
                }))
            }

            ErrorKind::IncorrectNumberOfParameters { expected, actual } => Some(KnownError::new(
                user_facing_errors::common::IncorrectNumberOfParameters {
                    expected: *expected,
                    actual: *actual,
                },
            )),
            ErrorKind::QueryParameterLimitExceeded(message) => Some(KnownError::new(
                user_facing_errors::query_engine::QueryParameterLimitExceeded {
                    message: message.clone(),
                },
            )),
            _ => None,
        };

        ConnectorError {
            user_facing_error,
            kind,
        }
    }
}

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("Unique constraint failed: {}", constraint)]
    UniqueConstraintViolation { constraint: DatabaseConstraint },

    #[error("Null constraint failed: {}", constraint)]
    NullConstraintViolation { constraint: DatabaseConstraint },

    #[error("Foreign key constraint failed")]
    ForeignKeyConstraintViolation { constraint: DatabaseConstraint },

    #[error("Record does not exist.")]
    RecordDoesNotExist,

    #[error("Column '{}' does not exist.", column)]
    ColumnDoesNotExist { column: String },

    #[error("Table '{}' does not exist.", table)]
    TableDoesNotExist { table: String },

    #[error("Error creating a database connection. ({})", _0)]
    ConnectionError(anyhow::Error),

    #[error("Error querying the database: {}", _0)]
    QueryError(Box<dyn std::error::Error + Send + Sync>),

    #[error("The provided arguments are not supported.")]
    InvalidConnectionArguments,

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
        child_name: String,
    },

    #[error("Conversion error: {}", _0)]
    ConversionError(anyhow::Error),

    #[error("Conversion error: {}", _0)]
    InternalConversionError(String),

    #[error("Database creation error: {}", _0)]
    DatabaseCreationError(&'static str),

    #[error("Database '{}' does not exist.", db_name)]
    DatabaseDoesNotExist { db_name: String },

    #[error("Access denied to database '{}'", db_name)]
    DatabaseAccessDenied { db_name: String },

    #[error("Authentication failed for user '{}'", user)]
    AuthenticationFailed { user: String },

    #[error("Database error. error code: {}, error message: {}", code, message)]
    RawError { code: String, message: String },

    #[error("{}", details)]
    InvalidDatabaseUrl { details: String, url: String },

    #[error("Unsupported connector feature: {0}")]
    UnsupportedFeature(String),

    #[error("Multiple errors occurred: {}", 0)]
    MultiError(MultiError),

    #[error(
        "Incorrect number of parameters given to a statement. Expected {}: got: {}.",
        expected,
        actual
    )]
    IncorrectNumberOfParameters { expected: usize, actual: usize },

    #[error("Server terminated the connection.")]
    ConnectionClosed,

    #[error("Transaction aborted: {}", message)]
    TransactionAborted { message: String },

    #[error("{}", message)]
    TransactionAlreadyClosed { message: String },

    #[error("The query parameter limit supported by your database is exceeded: {0}.")]
    QueryParameterLimitExceeded(String),
}

impl From<DomainError> for ConnectorError {
    fn from(e: DomainError) -> ConnectorError {
        ConnectorError::from_kind(ErrorKind::DomainError(e))
    }
}

#[derive(Debug)]
pub struct MultiError {
    pub errors: Vec<ErrorKind>,
}

impl Display for MultiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let errors = self
            .errors
            .iter()
            .enumerate()
            .map(|(i, err)| format!("{}) {}", i + 1, err))
            .collect_vec();

        write!(f, "{}", errors.join("\n"))
    }
}
