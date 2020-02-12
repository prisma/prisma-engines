use crate::filter::Filter;
use failure::{Error, Fail};
use prisma_models::prelude::DomainError;
use user_facing_errors::{query_engine::DatabaseConstraint, KnownError};

#[derive(Debug, Fail)]
#[fail(display = "{}", kind)]
pub struct ConnectorError {
    /// An optional error already rendered for users in case the migration core does not handle it.
    pub user_facing_error: Option<KnownError>,
    /// The error information for internal use.
    pub kind: ErrorKind,
}

impl ConnectorError {
    pub fn from_kind(kind: ErrorKind) -> Self {
        let user_facing_error = match &kind {
            ErrorKind::NullConstraintViolation { constraint } => Some(
                KnownError::new(user_facing_errors::query_engine::NullConstraintViolation {
                    constraint: constraint.to_owned(),
                })
                .unwrap(),
            ),
            _ => None,
        };

        ConnectorError {
            user_facing_error,
            kind,
        }
    }
}

#[derive(Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Unique constraint failed: {}", constraint)]
    UniqueConstraintViolation { constraint: DatabaseConstraint },

    #[fail(display = "Null constraint failed: {}", constraint)]
    NullConstraintViolation { constraint: DatabaseConstraint },

    #[fail(display = "Foreign key constraint failed")]
    ForeignKeyConstraintViolation { constraint: DatabaseConstraint },

    #[fail(display = "Record does not exist.")]
    RecordDoesNotExist,

    #[fail(display = "Column does not exist")]
    ColumnDoesNotExist,

    #[fail(display = "Error creating a database connection.")]
    ConnectionError(Error),

    #[fail(display = "Error querying the database: {}", _0)]
    QueryError(Box<dyn std::error::Error + Send + Sync>),

    #[fail(display = "The provided arguments are not supported.")]
    InvalidConnectionArguments,

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
        child_name: String,
    },

    #[fail(display = "Conversion error: {}", _0)]
    ConversionError(Error),

    #[fail(display = "Conversion error: {}", _0)]
    InternalConversionError(String),

    #[fail(display = "Database creation error: {}", _0)]
    DatabaseCreationError(&'static str),

    #[fail(display = "Database '{}' does not exist.", db_name)]
    DatabaseDoesNotExist { db_name: String },

    #[fail(display = "Access denied to database '{}'", db_name)]
    DatabaseAccessDenied { db_name: String },

    #[fail(display = "Authentication failed for user '{}'", user)]
    AuthenticationFailed { user: String },

    #[fail(display = "Database error. error code: {}, error message: {}", code, message)]
    RawError { code: String, message: String },
}

impl From<DomainError> for ConnectorError {
    fn from(e: DomainError) -> ConnectorError {
        ConnectorError::from_kind(ErrorKind::DomainError(e))
    }
}
