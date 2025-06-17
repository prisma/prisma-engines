use crate::error::{DatabaseConstraint, Error, ErrorKind};
use thiserror::Error;

/// This type represents MSSQL server error.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
#[error("ERROR {}: {}", code_name, message)]
pub struct MssqlError {
    pub code: Option<u32>,
    pub code_name: String,
    pub message: String,
}

impl From<MssqlError> for Error {
    fn from(error: MssqlError) -> Self {
        match error.code {
            Some(3902 | 3903 | 3971) => {
                let kind = ErrorKind::TransactionAlreadyClosed(error.message.clone());

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(8169) => {
                let kind = ErrorKind::conversion(error.message.clone());

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(18456) => {
                let user = error.message.split('\'').nth(1).into();
                let kind = ErrorKind::AuthenticationFailed { user };

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(4060) => {
                let db_name = error.message.split('"').nth(1).into();
                let kind = ErrorKind::DatabaseDoesNotExist { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(515) => {
                let constraint = error
                    .message
                    .split_whitespace()
                    .nth(7)
                    .and_then(|s| s.split('\'').nth(1))
                    .map(|s| DatabaseConstraint::fields(Some(s)))
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(1801) => {
                let db_name = error.message.split('\'').nth(1).into();
                let kind = ErrorKind::DatabaseAlreadyExists { db_name };

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(2627) => {
                let constraint = error
                    .message
                    .split(". ")
                    .nth(1)
                    .and_then(|s| s.split(' ').next_back())
                    .and_then(|s| s.split('\'').nth(1))
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(547) => {
                let constraint = error
                    .message
                    .split('.')
                    .next()
                    .and_then(|s| s.split_whitespace().last())
                    .and_then(|s| s.split('\"').nth(1))
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::ForeignKeyConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(1505) => {
                let constraint = error
                    .message
                    .split('\'')
                    .nth(3)
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(2601) => {
                let constraint = error
                    .message
                    .split_whitespace()
                    .nth(11)
                    .and_then(|s| s.split('\'').nth(1))
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(2628) => {
                let column = error.message.split('\'').nth(3).into();
                let kind = ErrorKind::LengthMismatch { column };

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(208) => {
                let table = error
                    .message
                    .split_whitespace()
                    .nth(3)
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::TableDoesNotExist { table };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(207) => {
                let column = error
                    .message
                    .split_whitespace()
                    .nth(3)
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::ColumnNotFound { column };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(1205) => {
                let mut builder = Error::builder(ErrorKind::TransactionWriteConflict);

                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            Some(5828) => {
                let mut builder = Error::builder(ErrorKind::TooManyConnections(error.clone().into()));
                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
            _ => {
                let kind = ErrorKind::QueryError(error.clone().into());

                let mut builder = Error::builder(kind);
                builder.set_original_code(error.code_name);
                builder.set_original_message(error.message);

                builder.build()
            }
        }
    }
}
