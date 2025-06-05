#[cfg(feature = "mysql")]
use quaint::error::MysqlError;

#[cfg(feature = "postgresql")]
use quaint::error::PostgresError;

#[cfg(feature = "sqlite")]
use quaint::error::SqliteError;
use serde::Deserialize;

#[cfg(feature = "postgresql")]
#[derive(Deserialize)]
#[serde(remote = "PostgresError")]
pub struct PostgresErrorDef {
    code: String,
    message: String,
    severity: String,
    detail: Option<String>,
    column: Option<String>,
    hint: Option<String>,
}

#[cfg(feature = "mysql")]
#[derive(Deserialize)]
#[serde(remote = "MysqlError")]
pub struct MysqlErrorDef {
    pub code: u16,
    pub message: String,
    pub state: String,
}

#[cfg(feature = "sqlite")]
#[derive(Deserialize)]
#[serde(remote = "SqliteError", rename_all = "camelCase")]
pub struct SqliteErrorDef {
    pub extended_code: i32,
    pub message: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
/// Wrapper for JS-side errors
pub(crate) enum DriverAdapterError {
    /// Unexpected JS exception
    GenericJs {
        id: i32,
    },
    UnsupportedNativeDataType {
        #[serde(rename = "type")]
        native_type: String,
    },
    InvalidIsolationLevel {
        level: String,
    },
    LengthMismatch {
        column: Option<String>,
    },
    UniqueConstraintViolation {
        constraint: DriverAdapterConstraint,
    },
    NullConstraintViolation {
        constraint: DriverAdapterConstraint,
    },
    ForeignKeyConstraintViolation {
        constraint: DriverAdapterConstraint,
    },
    DatabaseDoesNotExist {
        db: Option<String>,
    },
    DatabaseAlreadyExists {
        db: Option<String>,
    },
    DatabaseAccessDenied {
        db: Option<String>,
    },
    AuthenticationFailed {
        user: Option<String>,
    },
    TransactionWriteConflict,

    TableDoesNotExist {
        table: Option<String>,
    },
    ColumnNotFound {
        column: Option<String>,
    },
    TooManyConnections {
        cause: String,
    },
    ValueOutOfRange {
        cause: String,
    },
    MissingFullTextSearchIndex,
    #[cfg(feature = "postgresql")]
    #[serde(rename = "postgres")]
    Postgres(#[serde(with = "PostgresErrorDef")] PostgresError),
    #[cfg(feature = "mysql")]
    #[serde(rename = "mysql")]
    Mysql(#[serde(with = "MysqlErrorDef")] MysqlError),
    #[cfg(feature = "sqlite")]
    #[serde(rename = "sqlite")]
    Sqlite(#[serde(with = "SqliteErrorDef")] SqliteError),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DriverAdapterConstraint {
    Fields(Vec<String>),
    Index(String),
    ForeignKey,
}

impl From<DriverAdapterConstraint> for quaint::error::DatabaseConstraint {
    fn from(value: DriverAdapterConstraint) -> Self {
        match value {
            DriverAdapterConstraint::Fields(fields) => quaint::error::DatabaseConstraint::Fields(fields),
            DriverAdapterConstraint::Index(index) => quaint::error::DatabaseConstraint::Index(index),
            DriverAdapterConstraint::ForeignKey => quaint::error::DatabaseConstraint::ForeignKey,
        }
    }
}
