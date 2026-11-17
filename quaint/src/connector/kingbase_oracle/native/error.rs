use kingbase_tokio_postgres::error::DbError;

use crate::{
    connector::kingbase_oracle::KingbaseOracleError,
    error::{Error, ErrorKind, NativeErrorKind},
};

impl From<&DbError> for KingbaseOracleError {
    fn from(value: &DbError) -> Self {
        KingbaseOracleError {
            code: value.code().code().to_owned(),
            severity: value.severity().to_owned(),
            message: value.message().to_owned(),
            detail: value.detail().map(ToOwned::to_owned),
            column: value.column().map(ToOwned::to_owned),
            constraint: value.constraint().map(ToOwned::to_owned),
            hint: value.hint().map(ToOwned::to_owned),
        }
    }
}

/// Converts PostgreSQL-wire driver errors without defining the global
/// `From<kingbase_tokio_postgres::Error> for quaint::Error` implementation a
/// second time when both Kingbase modes are enabled.
pub(crate) fn convert_driver_error(error: kingbase_tokio_postgres::Error) -> Error {
    if error.is_closed() {
        return Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionClosed)).build();
    }

    if let Some(db_error) = error.as_db_error() {
        return KingbaseOracleError::from(db_error).into();
    }

    if let Some(tls_error) = try_extracting_tls_error(&error) {
        return tls_error;
    }

    if let Some(io_error) = try_extracting_io_error(&error) {
        return io_error;
    }

    if let Some(uuid_error) = try_extracting_uuid_error(&error) {
        return uuid_error;
    }

    let reason = error.to_string();
    let code = error.code().map(|code| code.code().to_owned());

    if reason == "error connecting to server: timed out" {
        let mut builder = Error::builder(ErrorKind::Native(NativeErrorKind::ConnectTimeout));
        if let Some(code) = code {
            builder.set_original_code(code);
        }
        builder.set_original_message(reason);
        return builder.build();
    }

    if reason.starts_with("error performing TLS handshake") {
        use std::error::Error as _;

        let message = error
            .source()
            .map(|source| format!("{reason}: {source}"))
            .unwrap_or_else(|| reason.clone());
        let mut builder = Error::builder(ErrorKind::Native(NativeErrorKind::TlsError { message }));
        if let Some(code) = code {
            builder.set_original_code(code);
        }
        builder.set_original_message(reason);
        return builder.build();
    }

    let mut builder = Error::builder(ErrorKind::QueryError(error.into()));
    if let Some(code) = code {
        builder.set_original_code(code);
    }
    builder.set_original_message(reason);
    builder.build()
}

fn try_extracting_uuid_error(error: &kingbase_tokio_postgres::Error) -> Option<Error> {
    use std::error::Error as _;

    error
        .source()
        .and_then(|source| source.downcast_ref::<uuid::Error>())
        .map(|error| ErrorKind::UUIDError(error.to_string()))
        .map(|kind| Error::builder(kind).build())
}

fn try_extracting_tls_error(error: &kingbase_tokio_postgres::Error) -> Option<Error> {
    use std::error::Error as _;

    error
        .source()
        .and_then(|source| source.downcast_ref::<native_tls::Error>())
        .map(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: error.to_string(),
            }))
            .build()
        })
}

fn try_extracting_io_error(error: &kingbase_tokio_postgres::Error) -> Option<Error> {
    use std::error::Error as _;

    error
        .source()
        .and_then(|source| source.downcast_ref::<std::io::Error>())
        .map(|error| {
            ErrorKind::Native(NativeErrorKind::ConnectionError(Box::new(std::io::Error::new(
                error.kind(),
                error.to_string(),
            ))))
        })
        .map(|kind| Error::builder(kind).build())
}
