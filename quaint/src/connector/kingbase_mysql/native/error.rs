use kingbase_tokio_postgres::error::DbError;

use crate::{
    connector::kingbase_mysql::error::KingbaseError,
    error::{Error, ErrorKind, NativeErrorKind},
};

impl From<&DbError> for KingbaseError {
    fn from(value: &DbError) -> Self {
        KingbaseError {
            code: value.code().code().to_string(),
            severity: value.severity().to_string(),
            message: value.message().to_string(),
            detail: value.detail().map(ToString::to_string),
            column: value.column().map(ToString::to_string),
            constraint: value.constraint().map(ToString::to_string),
            hint: value.hint().map(ToString::to_string),
        }
    }
}

impl From<kingbase_tokio_postgres::error::Error> for Error {
    fn from(e: kingbase_tokio_postgres::error::Error) -> Error {
        if e.is_closed() {
            return Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionClosed)).build();
        }

        if let Some(db_error) = e.as_db_error() {
            return KingbaseError::from(db_error).into();
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
                let mut builder = Error::builder(ErrorKind::Native(NativeErrorKind::ConnectTimeout));

                if let Some(code) = code {
                    builder.set_original_code(code);
                };

                builder.set_original_message(reason);
                builder.build()
            } // sigh...
            // https://github.com/sfackler/rust-postgres/blob/0c84ed9f8201f4e5b4803199a24afa2c9f3723b2/tokio-postgres/src/connect_tls.rs#L37
            // `kingbase-tokio-postgres` may omit the detailed handshake reason from
            // the outer error (for example, it reports just
            // `error performing TLS handshake` when the server has TLS disabled).
            // Match the stable prefix so these failures are still exposed as TLS
            // errors instead of generic query errors.
            reason if reason.starts_with("error performing TLS handshake") => {
                use std::error::Error as _;

                let message = e
                    .source()
                    .map(|source| format!("{reason}: {source}"))
                    .unwrap_or_else(|| reason.to_owned());
                let mut builder = Error::builder(ErrorKind::Native(NativeErrorKind::TlsError { message }));

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

fn try_extracting_uuid_error(err: &kingbase_tokio_postgres::error::Error) -> Option<Error> {
    use std::error::Error as _;

    err.source()
        .and_then(|err| err.downcast_ref::<uuid::Error>())
        .map(|err| ErrorKind::UUIDError(format!("{err}")))
        .map(|kind| Error::builder(kind).build())
}

fn try_extracting_tls_error(err: &kingbase_tokio_postgres::error::Error) -> Option<Error> {
    use std::error::Error as _;

    err.source()
        .and_then(|err| err.downcast_ref::<native_tls::Error>())
        .map(|err| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: err.to_string(),
            }))
            .build()
        })
}

fn try_extracting_io_error(err: &kingbase_tokio_postgres::error::Error) -> Option<Error> {
    use std::error::Error as _;

    err.source()
        .and_then(|err| err.downcast_ref::<std::io::Error>())
        .map(|err| {
            ErrorKind::Native(NativeErrorKind::ConnectionError(Box::new(std::io::Error::new(
                err.kind(),
                format!("{err}"),
            ))))
        })
        .map(|kind| Error::builder(kind).build())
}
