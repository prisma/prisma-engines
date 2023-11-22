use tokio_postgres::error::DbError;

use crate::{
    connector::postgres::error::PostgresError,
    error::{Error, ErrorKind},
};

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
