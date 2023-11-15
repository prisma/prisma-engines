use crate::error::{DatabaseConstraint, Error, ErrorKind};
use thiserror::Error;

// This is a partial copy of the `mysql_async::Error` using only the enum variant used by Prisma.
// This avoids pulling in `mysql_async`, which would break Wasm compilation.
#[derive(Debug, Error)]
enum MysqlAsyncError {
    #[error("Server error: `{}'", _0)]
    Server(#[source] MysqlError),
}

/// This type represents MySql server error.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
#[error("ERROR {} ({}): {}", state, code, message)]
pub struct MysqlError {
    pub code: u16,
    pub message: String,
    pub state: String,
}

impl From<MysqlError> for Error {
    fn from(error: MysqlError) -> Self {
        let code = error.code;
        match code {
            1062 => {
                let constraint = error
                    .message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('\'').nth(1))
                    .and_then(|s| s.split('.').last())
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1451 | 1452 => {
                let constraint = error
                    .message
                    .split_whitespace()
                    .nth(17)
                    .and_then(|s| s.split('`').nth(1))
                    .map(|s| DatabaseConstraint::fields(Some(s)))
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::ForeignKeyConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1263 => {
                let constraint = error
                    .message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('\'').nth(1))
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1264 => {
                let mut builder = Error::builder(ErrorKind::ValueOutOfRange {
                    message: error.message.clone(),
                });

                builder.set_original_code(code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            1364 | 1048 => {
                let constraint = error
                    .message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('\'').nth(1))
                    .map(|s| DatabaseConstraint::fields(Some(s)))
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1049 => {
                let db_name = error
                    .message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseDoesNotExist { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1007 => {
                let db_name = error
                    .message
                    .split_whitespace()
                    .nth(3)
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAlreadyExists { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1044 => {
                let db_name = error
                    .message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAccessDenied { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1045 => {
                let user = error
                    .message
                    .split_whitespace()
                    .nth(4)
                    .and_then(|s| s.split('@').next())
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::AuthenticationFailed { user };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1146 => {
                let table = error
                    .message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('\'').nth(1))
                    .and_then(|s| s.split('.').last())
                    .into();

                let kind = ErrorKind::TableDoesNotExist { table };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1054 => {
                let column = error
                    .message
                    .split_whitespace()
                    .nth(2)
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let mut builder = Error::builder(ErrorKind::ColumnNotFound { column });

                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
            1406 => {
                let column = error
                    .message
                    .split_whitespace()
                    .flat_map(|s| s.split('\''))
                    .nth(6)
                    .into();

                let kind = ErrorKind::LengthMismatch { column };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            1191 => {
                let kind = ErrorKind::MissingFullTextSearchIndex;
                let mut builder = Error::builder(kind);

                builder.set_original_code(code.to_string());
                builder.set_original_message(error.message);

                builder.build()
            }
            1213 => {
                let mut builder = Error::builder(ErrorKind::TransactionWriteConflict);
                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);
                builder.build()
            }
            _ => {
                let kind = ErrorKind::QueryError(
                    MysqlAsyncError::Server(MysqlError {
                        message: error.message.clone(),
                        code,
                        state: error.state.clone(),
                    })
                    .into(),
                );

                let mut builder = Error::builder(kind);
                builder.set_original_code(format!("{code}"));
                builder.set_original_message(error.message);

                builder.build()
            }
        }
    }
}
