#[cfg(feature = "mysql")]
use quaint::error::MysqlError;

#[cfg(feature = "postgresql")]
use quaint::error::PostgresError;

#[cfg(feature = "sqlite")]
use quaint::error::SqliteError;

#[cfg(feature = "mssql")]
use quaint::error::MssqlError;

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

#[cfg(feature = "mssql")]
#[derive(Deserialize)]
#[serde(remote = "MssqlError", rename_all = "camelCase")]
pub struct MssqlErrorDef {
    pub code: u32,
    pub message: String,
}

/// Wrapper for JS-side errors
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DriverAdapterError {
    pub original_message: Option<String>,
    pub original_code: Option<String>,
    #[serde(flatten)]
    pub mapped: MappedDriverAdapterError,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
pub(crate) enum MappedDriverAdapterError {
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
        constraint: Option<DriverAdapterConstraint>,
    },
    NullConstraintViolation {
        constraint: Option<DriverAdapterConstraint>,
    },
    ForeignKeyConstraintViolation {
        constraint: Option<DriverAdapterConstraint>,
    },
    DatabaseNotReachable {
        host: Option<String>,
        port: Option<u16>,
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
    ConnectionClosed,
    TlsConnectionError {
        reason: String,
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
    TransactionAlreadyClosed {
        cause: String,
    },
    #[cfg(feature = "postgresql")]
    #[serde(rename = "postgres")]
    Postgres(#[serde(with = "PostgresErrorDef")] PostgresError),
    #[cfg(feature = "mysql")]
    #[serde(rename = "mysql")]
    Mysql(#[serde(with = "MysqlErrorDef")] MysqlError),
    #[cfg(feature = "sqlite")]
    #[serde(rename = "sqlite")]
    Sqlite(#[serde(with = "SqliteErrorDef")] SqliteError),
    #[cfg(feature = "mssql")]
    #[serde(rename = "mssql")]
    Mssql(#[serde(with = "MssqlErrorDef")] MssqlError),
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
