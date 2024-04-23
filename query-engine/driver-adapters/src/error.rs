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
    GenericJs { id: i32 },
    UnsupportedNativeDataType {
        #[serde(rename = "type")]
        native_type: String,
    },
    #[cfg(feature = "postgresql")]
    Postgres(#[serde(with = "PostgresErrorDef")] PostgresError),
    #[cfg(feature = "mysql")]
    Mysql(#[serde(with = "MysqlErrorDef")] MysqlError),
    #[cfg(feature = "sqlite")]
    Sqlite(#[serde(with = "SqliteErrorDef")] SqliteError),
}
