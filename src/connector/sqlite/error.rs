use crate::error::*;
use libsqlite3_sys as ffi;
use rusqlite::types::FromSqlError;

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

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::ConstraintViolation,
                    extended_code: 2067,
                },
                Some(description),
            ) => {
                let constraint = description
                    .split(": ")
                    .nth(1)
                    .map(|s| s.split(", "))
                    .map(|i| i.flat_map(|s| s.split('.').last()))
                    .map(DatabaseConstraint::fields)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code("2067");
                builder.set_original_message(description);

                builder.build()
            }

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::ConstraintViolation,
                    extended_code: 1555,
                },
                Some(description),
            ) => {
                let constraint = description
                    .split(": ")
                    .nth(1)
                    .map(|s| s.split(", "))
                    .map(|i| i.flat_map(|s| s.split('.').last()))
                    .map(DatabaseConstraint::fields)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code("1555");
                builder.set_original_message(description);

                builder.build()
            }

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::ConstraintViolation,
                    extended_code: 1299,
                },
                Some(description),
            ) => {
                let constraint = description
                    .split(": ")
                    .nth(1)
                    .map(|s| s.split(", "))
                    .map(|i| i.flat_map(|s| s.split('.').last()))
                    .map(DatabaseConstraint::fields)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code("1299");
                builder.set_original_message(description);

                builder.build()
            }

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::ConstraintViolation,
                    extended_code: 787,
                },
                Some(description),
            ) => {
                let mut builder = Error::builder(ErrorKind::ForeignKeyConstraintViolation {
                    constraint: DatabaseConstraint::ForeignKey,
                });

                builder.set_original_code("787");
                builder.set_original_message(description);

                builder.build()
            }

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::DatabaseBusy,
                    extended_code,
                },
                description,
            ) => {
                let mut builder = Error::builder(ErrorKind::SocketTimeout);
                builder.set_original_code(format!("{}", extended_code));

                if let Some(description) = description {
                    builder.set_original_message(description);
                }

                builder.build()
            }

            rusqlite::Error::SqliteFailure(ffi::Error { extended_code, .. }, ref description) => match description {
                Some(d) if d.starts_with("no such table") => {
                    let table = d.split(": ").last().into();
                    let kind = ErrorKind::TableDoesNotExist { table };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{}", extended_code));
                    builder.set_original_message(d);

                    builder.build()
                }
                Some(d) if d.contains("has no column named") => {
                    let column = d.split(" has no column named ").last().into();
                    let kind = ErrorKind::ColumnNotFound { column };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{}", extended_code));
                    builder.set_original_message(d);

                    builder.build()
                }
                Some(d) if d.starts_with("no such column: ") => {
                    let column = d.split("no such column: ").last().into();
                    let kind = ErrorKind::ColumnNotFound { column };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{}", extended_code));
                    builder.set_original_message(d);

                    builder.build()
                }
                _ => {
                    let description = description.as_ref().map(|d| d.to_string());
                    let mut builder = Error::builder(ErrorKind::QueryError(e.into()));
                    builder.set_original_code(format!("{}", extended_code));

                    if let Some(description) = description {
                        builder.set_original_message(description);
                    }

                    builder.build()
                }
            },
            e => Error::builder(ErrorKind::QueryError(e.into())).build(),
        }
    }
}

impl From<FromSqlError> for Error {
    fn from(e: FromSqlError) -> Error {
        Error::builder(ErrorKind::ColumnReadFailure(e.into())).build()
    }
}
