use crate::error::{DatabaseConstraint, Error, ErrorKind};
use mysql_async as my;

impl From<my::Error> for Error {
    fn from(e: my::Error) -> Error {
        use my::ServerError;

        match e {
            my::Error::Io(my::IoError::Tls(err)) => Error::builder(ErrorKind::TlsError {
                message: err.to_string(),
            })
            .build(),
            my::Error::Io(io_error) => Error::builder(ErrorKind::ConnectionError(io_error.into())).build(),
            my::Error::Driver(e) => Error::builder(ErrorKind::QueryError(e.into())).build(),
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1062 => {
                let constraint = message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('\'').nth(1))
                    .and_then(|s| s.split('.').last())
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1451 || code == 1452 => {
                let constraint = message
                    .split_whitespace()
                    .nth(17)
                    .and_then(|s| s.split('`').nth(1))
                    .map(|s| DatabaseConstraint::fields(Some(s)))
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::ForeignKeyConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1263 => {
                let constraint = message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('\'').nth(1))
                    .map(ToString::to_string)
                    .map(DatabaseConstraint::Index)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1264 => {
                let mut builder = Error::builder(ErrorKind::ValueOutOfRange {
                    message: message.clone(),
                });

                builder.set_original_code(code.to_string());
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1364 || code == 1048 => {
                let constraint = message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('\'').nth(1))
                    .map(|s| DatabaseConstraint::fields(Some(s)))
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1049 => {
                let db_name = message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseDoesNotExist { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1007 => {
                let db_name = message
                    .split_whitespace()
                    .nth(3)
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAlreadyExists { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1044 => {
                let db_name = message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAccessDenied { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1045 => {
                let user = message
                    .split_whitespace()
                    .nth(4)
                    .and_then(|s| s.split('@').next())
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let kind = ErrorKind::AuthenticationFailed { user };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1146 => {
                let table = message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('\'').nth(1))
                    .and_then(|s| s.split('.').last())
                    .into();

                let kind = ErrorKind::TableDoesNotExist { table };
                let mut builder = Error::builder(kind);

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError { ref message, code, .. }) if code == 1054 => {
                let column = message
                    .split_whitespace()
                    .nth(2)
                    .and_then(|s| s.split('\'').nth(1))
                    .into();

                let mut builder = Error::builder(ErrorKind::ColumnNotFound { column });

                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError {
                ref message,
                code,
                state: _,
            }) if code == 1406 => {
                let column = message.split_whitespace().flat_map(|s| s.split('\'')).nth(6).into();

                let kind = ErrorKind::LengthMismatch { column };
                let mut builder = Error::builder(kind);

                builder.set_original_code(code.to_string());
                builder.set_original_message(message);

                builder.build()
            }
            my::Error::Server(ServerError {
                ref message,
                code,
                ref state,
            }) => {
                let kind = ErrorKind::QueryError(
                    my::Error::Server(ServerError {
                        message: message.clone(),
                        code,
                        state: state.clone(),
                    })
                    .into(),
                );

                let mut builder = Error::builder(kind);
                builder.set_original_code(format!("{}", code));
                builder.set_original_message(message);

                builder.build()
            }
            e => Error::builder(ErrorKind::QueryError(e.into())).build(),
        }
    }
}
