use std::fmt;

use crate::error::*;
use rusqlite::ffi;
use rusqlite::types::FromSqlError;

#[derive(Debug)]
pub struct SqliteError {
    pub extended_code: i32,
    pub message: Option<String>,
}

impl fmt::Display for SqliteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error code {}: {}",
            self.extended_code,
            ffi::code_to_str(self.extended_code)
        )
    }
}

impl std::error::Error for SqliteError {}

impl SqliteError {
    pub fn new(extended_code: i32, message: Option<String>) -> Self {
        Self { extended_code, message }
    }

    pub fn primary_code(&self) -> i32 {
        self.extended_code & 0xFF
    }
}

impl From<SqliteError> for Error {
    fn from(error: SqliteError) -> Self {
        match error {
            SqliteError {
                extended_code: ffi::SQLITE_CONSTRAINT_UNIQUE | ffi::SQLITE_CONSTRAINT_PRIMARYKEY,
                message: Some(description),
            } => {
                let constraint = description
                    .split(": ")
                    .nth(1)
                    .map(|s| s.split(", "))
                    .map(|i| i.flat_map(|s| s.split('.').last()))
                    .map(DatabaseConstraint::fields)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.extended_code.to_string());
                builder.set_original_message(description);

                builder.build()
            }

            SqliteError {
                extended_code: ffi::SQLITE_CONSTRAINT_NOTNULL,
                message: Some(description),
            } => {
                let constraint = description
                    .split(": ")
                    .nth(1)
                    .map(|s| s.split(", "))
                    .map(|i| i.flat_map(|s| s.split('.').last()))
                    .map(DatabaseConstraint::fields)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.extended_code.to_string());
                builder.set_original_message(description);

                builder.build()
            }

            SqliteError {
                extended_code: ffi::SQLITE_CONSTRAINT_FOREIGNKEY | ffi::SQLITE_CONSTRAINT_TRIGGER,
                message: Some(description),
            } => {
                let mut builder = Error::builder(ErrorKind::ForeignKeyConstraintViolation {
                    constraint: DatabaseConstraint::ForeignKey,
                });

                builder.set_original_code(error.extended_code.to_string());
                builder.set_original_message(description);

                builder.build()
            }

            SqliteError { extended_code, message } if error.primary_code() == ffi::SQLITE_BUSY => {
                let mut builder = Error::builder(ErrorKind::SocketTimeout);
                builder.set_original_code(format!("{extended_code}"));

                if let Some(description) = message {
                    builder.set_original_message(description);
                }

                builder.build()
            }

            SqliteError {
                extended_code,
                ref message,
            } => match message {
                Some(d) if d.starts_with("no such table") => {
                    let table = d.split(": ").last().into();
                    let kind = ErrorKind::TableDoesNotExist { table };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{extended_code}"));
                    builder.set_original_message(d);

                    builder.build()
                }
                Some(d) if d.contains("has no column named") => {
                    let column = d.split(" has no column named ").last().into();
                    let kind = ErrorKind::ColumnNotFound { column };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{extended_code}"));
                    builder.set_original_message(d);

                    builder.build()
                }
                Some(d) if d.starts_with("no such column: ") => {
                    let column = d.split("no such column: ").last().into();
                    let kind = ErrorKind::ColumnNotFound { column };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{extended_code}"));
                    builder.set_original_message(d);

                    builder.build()
                }
                _ => {
                    let description = message.as_ref().map(|d| d.to_string());
                    let mut builder = Error::builder(ErrorKind::QueryError(error.into()));
                    builder.set_original_code(format!("{extended_code}"));

                    if let Some(description) = description {
                        builder.set_original_message(description);
                    }

                    builder.build()
                }
            },
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        match e {
            rusqlite::Error::ToSqlConversionFailure(error) => match error.downcast::<Error>() {
                Ok(error) => *error,
                Err(error) => {
                    let mut builder = Error::builder(ErrorKind::QueryError(error));

                    builder.set_original_message("Could not interpret parameters in an SQLite query.");

                    builder.build()
                }
            },
            rusqlite::Error::InvalidQuery => {
                let mut builder = Error::builder(ErrorKind::QueryError(e.into()));

                builder.set_original_message(
                    "Could not interpret the query or its parameters. Check the syntax and parameter types.",
                );

                builder.build()
            }
            rusqlite::Error::ExecuteReturnedResults => {
                let mut builder = Error::builder(ErrorKind::QueryError(e.into()));
                builder.set_original_message("Execute returned results, which is not allowed in SQLite.");

                builder.build()
            }

            rusqlite::Error::QueryReturnedNoRows => Error::builder(ErrorKind::NotFound).build(),

            rusqlite::Error::SqliteFailure(ffi::Error { code: _, extended_code }, message) => {
                SqliteError::new(extended_code, message).into()
            }

            rusqlite::Error::SqlInputError {
                error: ffi::Error { extended_code, .. },
                msg,
                ..
            } => SqliteError::new(extended_code, Some(msg)).into(),

            e => Error::builder(ErrorKind::QueryError(e.into())).build(),
        }
    }
}

impl From<FromSqlError> for Error {
    fn from(e: FromSqlError) -> Error {
        Error::builder(ErrorKind::ColumnReadFailure(e.into())).build()
    }
}
