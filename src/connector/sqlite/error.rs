use crate::error::*;
use libsqlite3_sys as ffi;
use rusqlite::types::FromSqlError;

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound,

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
                    .map(|s| s.to_string()).collect();

                Error::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Fields(field_names)
                }
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
                    .map(|s| s.to_string()).collect();

                Error::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Fields(field_names)
                }
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
                    .map(|s| s.to_string()).collect();

                Error::NullConstraintViolation {
                    constraint: DatabaseConstraint::Fields(field_names)
                }
            }

            e => Error::QueryError(e.into()),
        }
    }
}

impl From<FromSqlError> for Error {
    fn from(e: FromSqlError) -> Error {
        Error::ColumnReadFailure(e.into())
    }
}
