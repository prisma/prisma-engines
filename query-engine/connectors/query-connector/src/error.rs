use itertools::Itertools;
use query_structure::prelude::DomainError;
use query_structure::Filter;
use std::fmt::Display;
use thiserror::Error;
use user_facing_errors::{query_engine::DatabaseConstraint, KnownError};

#[derive(Debug, Error)]
#[error("{}", kind)]
pub struct ConnectorError {
    /// An optional error already rendered for users.
    pub user_facing_error: Option<KnownError>,
    /// The error information for internal use.
    pub kind: ErrorKind,
    /// Whether an error is transient and should be retried.
    pub transient: bool,
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
                let details = user_facing_errors::invalid_connection_string_description(details);

                Some(KnownError::new(user_facing_errors::common::InvalidConnectionString {
                    details,
                }))
            }
            ErrorKind::ForeignKeyConstraintViolation { constraint } => {
                let field_name = match constraint {
                    DatabaseConstraint::Fields(fields) => fields.join(","),
                    DatabaseConstraint::Index(index) => format!("{index} (index)"),
                    DatabaseConstraint::ForeignKey => "foreign key".to_string(),
                    DatabaseConstraint::CannotParse => "(not available)".to_string(),
                };

                Some(KnownError::new(user_facing_errors::query_engine::ForeignKeyViolation {
                    field_name,
                }))
            }
            ErrorKind::ConversionError(message) => Some(KnownError::new(
                user_facing_errors::query_engine::InconsistentColumnData {
                    message: format!("{message}"),
                },
            )),
            ErrorKind::QueryInvalidInput(message) => Some(KnownError::new(
                user_facing_errors::query_engine::DatabaseAssertionViolation {
                    database_error: message.to_owned(),
                },
            )),
            ErrorKind::UnsupportedFeature(feature) => {
                Some(KnownError::new(user_facing_errors::query_engine::UnsupportedFeature {
                    feature: feature.clone(),
                }))
            }
            ErrorKind::MultiError(merror) => Some(KnownError::new(user_facing_errors::query_engine::MultiError {
                errors: format!("{merror}"),
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

            ErrorKind::MissingFullTextSearchIndex => Some(KnownError::new(
                user_facing_errors::query_engine::MissingFullTextSearchIndex {},
            )),
            ErrorKind::TransactionAborted { message } => Some(KnownError::new(
                user_facing_errors::query_engine::InteractiveTransactionError { error: message.clone() },
            )),
            ErrorKind::TransactionWriteConflict => Some(KnownError::new(
                user_facing_errors::query_engine::TransactionWriteConflict {},
            )),
            ErrorKind::TransactionAlreadyClosed { message } => {
                Some(KnownError::new(user_facing_errors::common::TransactionAlreadyClosed {
                    message: message.clone(),
                }))
            }
            ErrorKind::ConnectionClosed => Some(KnownError::new(user_facing_errors::common::ConnectionClosed)),
            ErrorKind::MongoReplicaSetRequired => Some(KnownError::new(
                user_facing_errors::query_engine::MongoReplicaSetRequired {},
            )),
            ErrorKind::RawDatabaseError { code, message } => Some(user_facing_errors::KnownError::new(
                user_facing_errors::query_engine::RawQueryFailed {
                    code: code.clone(),
                    message: message.clone(),
                },
            )),
            ErrorKind::ExternalError(id) => Some(user_facing_errors::KnownError::new(
                user_facing_errors::query_engine::ExternalError { id: id.to_owned() },
            )),
            ErrorKind::RecordDoesNotExist { cause } => Some(KnownError::new(
                user_facing_errors::query_engine::RecordRequiredButNotFound { cause: cause.clone() },
            )),

            ErrorKind::TooManyConnections(e) => Some(user_facing_errors::KnownError::new(
                user_facing_errors::query_engine::TooManyConnections {
                    message: format!("{}", e),
                },
            )),
            _ => None,
        };

        ConnectorError {
            user_facing_error,
            kind,
            transient: false,
        }
    }

    pub fn set_transient(&mut self, transient: bool) {
        self.transient = transient;
    }

    pub fn is_transient(&self) -> bool {
        self.transient
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

    #[error("Record does not exist: {cause}")]
    RecordDoesNotExist { cause: String },

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

    #[error("Invalid input provided to query: {}", _0)]
    QueryInvalidInput(String),

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
    RawDatabaseError { code: String, message: String },

    #[error("Raw API error: {0}")]
    RawApiError(String),

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

    #[error("Transaction write conflict")]
    TransactionWriteConflict,

    #[error("ROLLBACK statement has no corresponding BEGIN statement")]
    RollbackWithoutBegin,

    #[error("The query parameter limit supported by your database is exceeded: {0}.")]
    QueryParameterLimitExceeded(String),

    #[error("Cannot find a fulltext index to use for the search")]
    MissingFullTextSearchIndex,

    #[error("Replica Set required for Transactions")]
    MongoReplicaSetRequired,

    #[error("Unsupported connector: {0}")]
    UnsupportedConnector(String),

    #[error("External connector error")]
    ExternalError(i32),

    #[error("Invalid driver adapter: {0}")]
    InvalidDriverAdapter(String),

    #[error("Too many DB connections opened: {}", _0)]
    TooManyConnections(Box<dyn std::error::Error + Send + Sync>),

    #[error("Failed to parse database version: {}. Reason: {}", version, reason)]
    UnexpectedDatabaseVersion { version: String, reason: String },
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
