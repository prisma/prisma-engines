use quaint::error::{MysqlError, PostgresError, SqliteError};
use serde::Deserialize;

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

#[derive(Deserialize)]
#[serde(remote = "MysqlError")]
pub struct MysqlErrorDef {
    pub code: u16,
    pub message: String,
    pub state: String,
}

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
    Postgres(#[serde(with = "PostgresErrorDef")] PostgresError),
    Mysql(#[serde(with = "MysqlErrorDef")] MysqlError),
    Sqlite(#[serde(with = "SqliteErrorDef")] SqliteError),
}
