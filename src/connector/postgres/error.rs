use crate::error::{DatabaseConstraint, Error, ErrorKind, Name};

impl From<tokio_postgres::error::Error> for Error {
    fn from(e: tokio_postgres::error::Error) -> Error {
        use tokio_postgres::error::DbError;

        if e.is_closed() {
            return Error::builder(ErrorKind::ConnectionClosed).build();
        }

        match e.code().map(|c| c.code()) {
            Some(code) if code == "22001" => {
                let code = code.to_string();

                let mut builder = Error::builder(ErrorKind::LengthMismatch {
                    column: Name::Unavailable,
                });

                builder.set_original_code(code);

                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());
                if let Some(db_error) = db_error {
                    builder.set_original_message(db_error.to_string());
                }

                builder.build()
            }
            Some(code) if code == "23505" => {
                let code = code.to_string();

                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());
                let detail = db_error.as_ref().and_then(|e| e.detail()).map(ToString::to_string);

                let constraint = detail
                    .as_ref()
                    .and_then(|d| d.split(")=(").next())
                    .and_then(|d| d.split(" (").nth(1).map(|s| s.replace("\"", "")))
                    .map(|s| DatabaseConstraint::fields(s.split(", ")))
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code);

                if let Some(detail) = detail {
                    builder.set_original_message(detail);
                }

                builder.build()
            }
            // Even lipstick will not save this...
            Some(code) if code == "23502" => {
                let code = code.to_string();

                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());
                let detail = db_error.as_ref().and_then(|e| e.detail()).map(ToString::to_string);

                let constraint = db_error
                    .as_ref()
                    .map(|e| e.column())
                    .map(DatabaseConstraint::fields)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code);

                if let Some(detail) = detail {
                    builder.set_original_message(detail);
                }

                builder.build()
            }
            Some(code) if code == "23503" => {
                let code = code.to_string();
                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());

                match db_error.as_ref().and_then(|e| e.column()) {
                    Some(column) => {
                        let mut builder = Error::builder(ErrorKind::ForeignKeyConstraintViolation {
                            constraint: DatabaseConstraint::fields(Some(column)),
                        });

                        builder.set_original_code(code);

                        if let Some(message) = db_error.as_ref().map(|e| e.message()) {
                            builder.set_original_message(message);
                        }

                        builder.build()
                    }
                    None => {
                        let constraint = db_error
                            .as_ref()
                            .map(|e| e.message())
                            .and_then(|e| e.split_whitespace().nth(10))
                            .and_then(|s| s.split('"').nth(1))
                            .map(ToString::to_string)
                            .map(DatabaseConstraint::Index)
                            .unwrap_or(DatabaseConstraint::CannotParse);

                        let kind = ErrorKind::ForeignKeyConstraintViolation { constraint };
                        let mut builder = Error::builder(kind);

                        builder.set_original_code(code);

                        if let Some(message) = db_error.as_ref().map(|e| e.message()) {
                            builder.set_original_message(message);
                        }

                        builder.build()
                    }
                }
            }
            Some(code) if code == "3D000" => {
                let code = code.to_string();
                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());
                let message = db_error.as_ref().map(|e| e.message());

                let db_name = message
                    .as_ref()
                    .and_then(|s| s.split_whitespace().nth(1))
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseDoesNotExist { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code);

                if let Some(message) = message {
                    builder.set_original_message(message);
                }

                builder.build()
            }
            Some(code) if code == "28P01" => {
                let code = code.to_string();
                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());
                let message = db_error.as_ref().map(|e| e.message());

                let user = message
                    .as_ref()
                    .and_then(|m| m.split_whitespace().last())
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::AuthenticationFailed { user };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code);

                if let Some(message) = message {
                    builder.set_original_message(message);
                }

                builder.build()
            }
            Some(code) if code == "42P01" => {
                let code = code.to_string();
                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());
                let message = db_error.as_ref().map(|e| e.message());

                let table = message
                    .as_ref()
                    .and_then(|m| m.split_whitespace().nth(1))
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::TableDoesNotExist { table };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code);

                if let Some(message) = message {
                    builder.set_original_message(message);
                }

                builder.build()
            }
            Some(code) if code == "42703" => {
                let code = code.to_string();
                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());
                let message = db_error.as_ref().map(|e| e.message());

                let column = message
                    .as_ref()
                    .and_then(|m| m.split_whitespace().nth(1))
                    .map(|s| s.split('\"'))
                    .and_then(|mut s| match (s.next(), s.next()) {
                        (Some(column), _) if !column.is_empty() => Some(column),
                        (_, Some(column)) if !column.is_empty() => Some(column),
                        (_, _) => None,
                    })
                    .into();

                let kind = ErrorKind::ColumnNotFound { column };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code);

                if let Some(message) = message {
                    builder.set_original_message(message);
                }

                builder.build()
            }

            Some(code) if code == "42P04" => {
                let code = code.to_string();
                let db_error = e.into_source().and_then(|e| e.downcast::<DbError>().ok());
                let message = db_error.as_ref().map(|e| e.message());

                let db_name = message
                    .as_ref()
                    .and_then(|m| m.split_whitespace().nth(1))
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAlreadyExists { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code);

                if let Some(message) = message {
                    builder.set_original_message(message);
                }

                builder.build()
            }
            code => {
                // This is necessary, on top of the other conversions, for the cases where a
                // native_tls error comes wrapped in a tokio_postgres error.
                if let Some(tls_error) = try_extracting_tls_error(&e) {
                    return tls_error;
                }

                // Same for IO errors.
                if let Some(io_error) = try_extracting_io_error(&e) {
                    return io_error;
                }

                let reason = format!("{}", e);

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
    }
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
        .map(|err| ErrorKind::ConnectionError(Box::new(std::io::Error::new(err.kind(), format!("{}", err)))))
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
            message: format!("{}", e),
        };

        Error::builder(kind).build()
    }
}
