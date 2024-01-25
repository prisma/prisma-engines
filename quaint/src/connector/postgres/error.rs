use std::fmt::{Display, Formatter};

use crate::error::{DatabaseConstraint, Error, ErrorKind, Name};

#[derive(Debug)]
pub struct PostgresError {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub detail: Option<String>,
    pub column: Option<String>,
    pub hint: Option<String>,
}

impl std::error::Error for PostgresError {}

impl Display for PostgresError {
    // copy of tokio_postgres::error::DbError::fmt
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}: {}", self.severity, self.message)?;
        if let Some(detail) = &self.detail {
            write!(fmt, "\nDETAIL: {}", detail)?;
        }
        if let Some(hint) = &self.hint {
            write!(fmt, "\nHINT: {}", hint)?;
        }
        Ok(())
    }
}

impl From<PostgresError> for Error {
    fn from(value: PostgresError) -> Self {
        match value.code.as_str() {
            "22001" => {
                let mut builder = Error::builder(ErrorKind::LengthMismatch {
                    column: Name::Unavailable,
                });

                builder.set_original_code(&value.code);
                builder.set_original_message(value.to_string());

                builder.build()
            }
            "23505" => {
                let constraint = value
                    .detail
                    .as_ref()
                    .and_then(|d| d.split(")=(").next())
                    .and_then(|d| d.split(" (").nth(1).map(|s| s.replace('\"', "")))
                    .map(|s| DatabaseConstraint::fields(s.split(", ")))
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);

                if let Some(detail) = value.detail {
                    builder.set_original_message(detail);
                }

                builder.build()
            }

            // Even lipstick will not save this...
            "23502" => {
                let constraint = DatabaseConstraint::fields(value.column);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);

                if let Some(detail) = value.detail {
                    builder.set_original_message(detail);
                }

                builder.build()
            }
            "23503" => match value.column {
                Some(column) => {
                    let mut builder = Error::builder(ErrorKind::ForeignKeyConstraintViolation {
                        constraint: DatabaseConstraint::fields(Some(column)),
                    });

                    builder.set_original_code(value.code);
                    builder.set_original_message(value.message);

                    builder.build()
                }
                None => {
                    let constraint = value
                        .message
                        .split_whitespace()
                        .nth(10)
                        .and_then(|s| s.split('"').nth(1))
                        .map(ToString::to_string)
                        .map(DatabaseConstraint::Index)
                        .unwrap_or(DatabaseConstraint::CannotParse);

                    let kind = ErrorKind::ForeignKeyConstraintViolation { constraint };
                    let mut builder = Error::builder(kind);

                    builder.set_original_code(value.code);
                    builder.set_original_message(value.message);

                    builder.build()
                }
            },
            "3D000" => {
                let db_name = value
                    .message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseDoesNotExist { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "28000" => {
                let db_name = value
                    .message
                    .split_whitespace()
                    .nth(5)
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAccessDenied { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "28P01" => {
                let message = value.message;

                let user = message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::AuthenticationFailed { user };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(message);

                builder.build()
            }
            "40001" => {
                let mut builder: crate::error::ErrorBuilder = Error::builder(ErrorKind::TransactionWriteConflict);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "42P01" => {
                let table = value
                    .message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::TableDoesNotExist { table };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "42703" => {
                let column = value
                    .message
                    .split_whitespace()
                    .nth(1)
                    .map(|s| s.split('\"'))
                    .and_then(|mut s| match (s.next(), s.next()) {
                        (Some(column), _) if !column.is_empty() => Some(column),
                        (_, Some(column)) if !column.is_empty() => Some(column),
                        (_, _) => None,
                    })
                    .into();

                let kind = ErrorKind::ColumnNotFound { column };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);
                builder.build()
            }

            "42P04" => {
                let db_name = value
                    .message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAlreadyExists { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }

            _ => {
                let code = value.code.to_owned();
                let message = value.to_string();
                let mut builder = Error::builder(ErrorKind::QueryError(value.into()));

                builder.set_original_code(code);
                builder.set_original_message(message);
                builder.build()
            }
        }
    }
}
