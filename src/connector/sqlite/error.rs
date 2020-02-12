use crate::error::*;
use libsqlite3_sys as ffi;
use rusqlite::types::FromSqlError;

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        match e {
            rusqlite::Error::QueryReturnedNoRows => Error::builder(ErrorKind::NotFound).build(),

            rusqlite::Error::SqliteFailure(
                ffi::Error {
                    code: ffi::ErrorCode::ConstraintViolation,
                    extended_code: 2067,
                },
                Some(description),
            ) => {
                let splitted: Vec<&str> = description.split(": ").collect();

                let field_names: Vec<String> = splitted[1]
                    .split(", ")
                    .map(|s| s.split(".").last().unwrap())
                    .map(|s| s.to_string())
                    .collect();

                let mut builder = Error::builder(ErrorKind::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Fields(field_names),
                });

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
                let splitted: Vec<&str> = description.split(": ").collect();

                let field_names: Vec<String> = splitted[1]
                    .split(", ")
                    .map(|s| s.split(".").last().unwrap())
                    .map(|s| s.to_string())
                    .collect();

                let mut builder = Error::builder(ErrorKind::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Fields(field_names),
                });

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
                let splitted: Vec<&str> = description.split(": ").collect();

                let field_names: Vec<String> = splitted[1]
                    .split(", ")
                    .map(|s| s.split(".").last().unwrap())
                    .map(|s| s.to_string())
                    .collect();

                let mut builder = Error::builder(ErrorKind::NullConstraintViolation {
                    constraint: DatabaseConstraint::Fields(field_names),
                });

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
                let mut builder = Error::builder(ErrorKind::Timeout("SQLite database is busy".into()));
                builder.set_original_code(format!("{}", extended_code));

                if let Some(description) = description {
                    builder.set_original_message(description);
                }

                builder.build()
            }

            rusqlite::Error::SqliteFailure(ffi::Error { extended_code, .. }, ref description) => {
                let description = description.as_ref().map(|d| d.to_string());
                let mut builder = Error::builder(ErrorKind::QueryError(e.into()));
                builder.set_original_code(format!("{}", extended_code));

                if let Some(description) = description {
                    builder.set_original_message(description);
                }

                builder.build()
            }
            e => Error::builder(ErrorKind::QueryError(e.into())).build(),
        }
    }
}

impl From<FromSqlError> for Error {
    fn from(e: FromSqlError) -> Error {
        Error::builder(ErrorKind::ColumnReadFailure(e.into())).build()
    }
}
