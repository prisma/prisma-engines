use crate::error::{DatabaseConstraint, Error, ErrorKind};
use thiserror::Error;

/// This type represents MSSQL server error.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
#[error("ERROR {}: {}", code, message)]
pub struct MssqlError {
    pub code: u32,
    pub message: String,
}

impl From<MssqlError> for Error {
    fn from(error: MssqlError) -> Self {
        match error.code {
            3902 | 3903 | 3971 => {
                let kind = ErrorKind::TransactionAlreadyClosed(error.message.clone());

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            8169 => {
                let kind = ErrorKind::conversion(error.message.clone());

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            18456 => {
                let user = error.message.split('\'').nth(1).into();
                let kind = ErrorKind::AuthenticationFailed { user };

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            4060 => {
                let db_name = error.message.split('"').nth(1).into();
                let kind = ErrorKind::DatabaseDoesNotExist { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            515 => {
                let constraint = error
                    .message
                    .split_whitespace()
                    .nth(7)
                    .and_then(|s| s.split('\'').nth(1))
                    .map(|s| DatabaseConstraint::fields(Some(s)))
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            1801 => {
                let db_name = error.message.split('\'').nth(1).into();
                let kind = ErrorKind::DatabaseAlreadyExists { db_name };

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            2627 => {
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

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            547 => {
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

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            1505 => {
                let constraint = error
                    .message
                    .split('\'')
                    .nth(3)
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            2601 => {
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

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            2628 => {
                let column = error.message.split('\'').nth(3).into();
                let kind = ErrorKind::LengthMismatch { column };

                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            208 => {
                let table = error
                    .message
                    .split_whitespace()
                    .nth(3)
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::TableDoesNotExist { table };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            207 => {
                let column = error
                    .message
                    .split_whitespace()
                    .nth(3)
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::ColumnNotFound { column };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            1205 => {
                let mut builder = Error::builder(ErrorKind::TransactionWriteConflict);

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            5828 => {
                let mut builder = Error::builder(ErrorKind::TooManyConnections(error.clone().into()));
                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            108 | 168 | 183 | 187 | 220 | 232 | 236 | 242 | 244 | 248 | 294 | 296 | 298 | 304 | 517 | 535 | 1007
            | 1080 | 2386 | 2568 | 2570 | 2579 | 2742 | 2950 | 3194 | 3250 | 3606 | 3995 | 4079 | 4867 | 6244
            | 6398 | 6937 | 6938 | 6960 | 7116 | 7135 | 7722 | 7810 | 7981 | 8115 | 8165 | 8351 | 8411 | 8727
            | 8729 | 8968 | 8991 | 9109 | 9204 | 9526 | 9527 | 9746 | 9813 | 9835 | 9838 | 9839 => {
                let mut builder = Error::builder(ErrorKind::value_out_of_range(error.message.clone()));

                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            _ => {
                let kind = ErrorKind::QueryError(error.clone().into());

                let mut builder = Error::builder(kind);
                builder.set_original_code(error.code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
        }
    }
}
