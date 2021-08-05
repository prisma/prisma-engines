use crate::UserFacingError;
use serde::Serialize;
use std::fmt::Display;
use user_facing_error_macros::*;

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P1000",
    message = "\
Authentication failed against database server at `{database_host}`, the provided database credentials for `{database_user}` are not valid.

Please make sure to provide valid database credentials for the database server at `{database_host}`."
)]
// **Notes**: Might vary for different data source, For example, SQLite has no concept of user accounts, and instead relies on the file system for all database permissions. This makes enforcing storage quotas difficult and enforcing user permissions impossible.
pub struct IncorrectDatabaseCredentials {
    /// Database host URI
    pub database_user: String,

    /// Database user name
    pub database_host: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P1001",
    message = "\
Can't reach database server at `{database_host}`:`{database_port}`

Please make sure your database server is running at `{database_host}`:`{database_port}`."
)]
pub struct DatabaseNotReachable {
    /// Database host URI
    pub database_host: String,

    /// Database port
    pub database_port: u16,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P1002",
    message = "\
The database server at `{database_host}`:`{database_port}` was reached but timed out.

Please try again.

Please make sure your database server is running at `{database_host}`:`{database_port}`.

Context: {context}
"
)]
pub struct DatabaseTimeout {
    /// Database host URI
    pub database_host: String,

    /// Database port
    pub database_port: String,

    /// Extra context
    pub context: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum DatabaseDoesNotExist {
    Sqlite {
        database_file_name: String,
        database_file_path: String,
    },
    Postgres {
        database_name: String,
        database_schema_name: String,
        database_host: String,
        database_port: u16,
    },
    Mysql {
        database_name: String,
        database_host: String,
        database_port: u16,
    },

    Mssql {
        database_name: String,
        database_host: String,
        database_port: u16,
    },
}

impl UserFacingError for DatabaseDoesNotExist {
    const ERROR_CODE: &'static str = "P1003";

    fn message(&self) -> String {
        match self {
            DatabaseDoesNotExist::Sqlite {
                database_file_name,
                database_file_path,
            } => format!(
                "Database {database_file_name} does not exist at {database_file_path}",
                database_file_name = database_file_name,
                database_file_path = database_file_path
            ),
            DatabaseDoesNotExist::Postgres {
                database_name,
                database_schema_name,
                database_host,
                database_port,
            } => format!(
                "Database `{database_name}.{database_schema_name}` does not exist on the database server at `{database_host}:{database_port}`.",
                database_name = database_name,
                database_schema_name = database_schema_name,
                database_host = database_host,
                database_port = database_port,
            ),
            DatabaseDoesNotExist::Mysql {
                database_name,
                database_host,
                database_port,
            } => format!(
                "Database `{database_name}` does not exist on the database server at `{database_host}:{database_port}`.",
                database_name = database_name,
                database_host = database_host,
                database_port = database_port,
            ),
            DatabaseDoesNotExist::Mssql {
                database_name,
                database_host,
                database_port,
            } => format!(
                "Database `{database_name}` does not exist on the database server at `{database_host}:{database_port}`.",
                database_name = database_name,
                database_host = database_host,
                database_port = database_port,
            ),
        }
    }
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P1008", message = "Operations timed out after `{time}`. Context: {context}")]
pub struct DatabaseOperationTimeout {
    /// Operation time in s or ms (if <1000ms)
    pub time: String,

    /// Extra context
    pub context: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P1009",
    message = "Database `{database_name}` already exists on the database server at `{database_host}:{database_port}`"
)]
pub struct DatabaseAlreadyExists {
    /// Database name, append `database_schema_name` when applicable
    /// `database_schema_name`: Database schema name (For Postgres for example)
    pub database_name: String,

    /// Database host URI
    pub database_host: String,

    /// Database port
    pub database_port: u16,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P1010",
    message = "User `{database_user}` was denied access on the database `{database_name}`"
)]
pub struct DatabaseAccessDenied {
    /// Database user name
    pub database_user: String,

    /// Database name, append `database_schema_name` when applicable
    /// `database_schema_name`: Database schema name (For Postgres for example)
    pub database_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P1011", message = "Error opening a TLS connection: {message}")]
pub struct TlsConnectionError {
    pub message: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P1012", message = "{full_error}")]
pub struct SchemaParserError {
    pub full_error: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P1013", message = "The provided database string is invalid. {details}")]
pub struct InvalidConnectionString {
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ModelKind {
    Table,
}

impl Display for ModelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
        }
    }
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P1014",
    message = "The underlying {kind} for model `{model}` does not exist."
)]
pub struct InvalidModel {
    pub model: String,
    pub kind: ModelKind,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P1015",
    message = "Your Prisma schema is using features that are not supported for the version of the database.\nDatabase version: {database_version}\nErrors:\n{errors}"
)]
pub struct DatabaseVersionIncompatibility {
    pub database_version: String,
    pub errors: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P1016",
    message = "Your raw query had an incorrect number of parameters. Expected: `{expected}`, actual: `{actual}`."
)]
pub struct IncorrectNumberOfParameters {
    pub expected: usize,
    pub actual: usize,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P1017", message = "Server has closed the connection.")]
pub struct ConnectionClosed;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UserFacingError;

    #[test]
    fn database_does_not_exist_formats_properly() {
        let sqlite_err = DatabaseDoesNotExist::Sqlite {
            database_file_path: "/tmp/dev.db".into(),
            database_file_name: "dev.db".into(),
        };

        assert_eq!(sqlite_err.message(), "Database dev.db does not exist at /tmp/dev.db");

        let mysql_err = DatabaseDoesNotExist::Mysql {
            database_name: "root".into(),
            database_host: "localhost".into(),
            database_port: 8888,
        };

        assert_eq!(
            mysql_err.message(),
            "Database `root` does not exist on the database server at `localhost:8888`."
        );
    }
}
