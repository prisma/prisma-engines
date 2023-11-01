use std::fmt::{Display, Formatter};

use tokio_postgres::error::DbError;

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
    // copy of DbError::fmt
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

impl From<&DbError> for PostgresError {
    fn from(value: &DbError) -> Self {
        PostgresError {
            code: value.code().code().to_string(),
            severity: value.severity().to_string(),
            message: value.message().to_string(),
            detail: value.detail().map(ToString::to_string),
            column: value.column().map(ToString::to_string),
            hint: value.hint().map(ToString::to_string),
        }
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

impl From<tokio_postgres::error::Error> for Error {
    fn from(e: tokio_postgres::error::Error) -> Error {
        if e.is_closed() {
            return Error::builder(ErrorKind::ConnectionClosed).build();
        }

        if let Some(db_error) = e.as_db_error() {
            return PostgresError::from(db_error).into();
        }

        if let Some(tls_error) = try_extracting_tls_error(&e) {
            return tls_error;
        }

        // Same for IO errors.
        if let Some(io_error) = try_extracting_io_error(&e) {
            return io_error;
        }

        if let Some(uuid_error) = try_extracting_uuid_error(&e) {
            return uuid_error;
        }

        let reason = format!("{e}");
        let code = e.code().map(|c| c.code());

        match reason.as_str() {
            "error connecting to server: timed out" => {
                let mut builder = Error::builder(ErrorKind::ConnectTimeout);

                if let Some(code) = code {
                    builder.set_original_code(code);
                };

                builder.set_original_message(reason);
                builder.build()
            } // sigh...
            // https://github.com/sfackler/rust-postgres/blob/0c84ed9f8201f4e5b4803199a24afa2c9f3723b2/tokio-postgres/src/connect_tls.rs#L37
            "error performing TLS handshake: server does not support TLS" => {
                let mut builder = Error::builder(ErrorKind::TlsError {
                    message: reason.clone(),
                });

                if let Some(code) = code {
                    builder.set_original_code(code);
                };

                builder.set_original_message(reason);
                builder.build()
            } // double sigh
            _ => {
                let code = code.map(|c| c.to_string());
                let mut builder = Error::builder(ErrorKind::QueryError(e.into()));

                if let Some(code) = code {
                    builder.set_original_code(code);
                };

                builder.set_original_message(reason);
                builder.build()
            }
        }
    }
}

fn try_extracting_uuid_error(err: &tokio_postgres::error::Error) -> Option<Error> {
    use std::error::Error as _;

    err.source()
        .and_then(|err| err.downcast_ref::<uuid::Error>())
        .map(|err| ErrorKind::UUIDError(format!("{err}")))
        .map(|kind| Error::builder(kind).build())
}

fn try_extracting_tls_error(err: &tokio_postgres::error::Error) -> Option<Error> {
    use std::error::Error;

    err.source()
        .and_then(|err| err.downcast_ref::<native_tls::Error>())
        .map(|err| err.into())
}

fn try_extracting_io_error(err: &tokio_postgres::error::Error) -> Option<Error> {
    use std::error::Error as _;

    err.source()
        .and_then(|err| err.downcast_ref::<std::io::Error>())
        .map(|err| ErrorKind::ConnectionError(Box::new(std::io::Error::new(err.kind(), format!("{err}")))))
        .map(|kind| Error::builder(kind).build())
}

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Error {
        Error::from(&e)
    }
}

impl From<&native_tls::Error> for Error {
    fn from(e: &native_tls::Error) -> Error {
        let kind = ErrorKind::TlsError {
            message: format!("{e}"),
        };

        Error::builder(kind).build()
    }
}
