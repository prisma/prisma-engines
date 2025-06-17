use crate::{
    connector::MssqlError,
    error::{Error, ErrorKind, NativeErrorKind},
};
use tiberius::error::IoErrorKind;

impl From<tiberius::error::Error> for Error {
    fn from(e: tiberius::error::Error) -> Error {
        match e {
            tiberius::error::Error::Io {
                kind: IoErrorKind::UnexpectedEof,
                message,
            } => {
                let mut builder = Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionClosed));
                builder.set_original_message(message);
                builder.build()
            }
            e @ tiberius::error::Error::Io { .. } => {
                Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionError(e.into()))).build()
            }
            tiberius::error::Error::Tls(message) => {
                let message = format!(
                    "The TLS settings didn't allow the connection to be established. Please review your connection string. (error: {message})"
                );

                Error::builder(ErrorKind::Native(NativeErrorKind::TlsError { message })).build()
            }
            tiberius::error::Error::Server(err) => MssqlError::from(err).into(),
            e => Error::builder(ErrorKind::QueryError(e.into())).build(),
        }
    }
}

impl From<tiberius::error::TokenError> for MssqlError {
    fn from(value: tiberius::error::TokenError) -> Self {
        MssqlError {
            code: Some(value.code()),
            code_name: value.code().to_string(),
            message: value.message().to_owned(),
        }
    }
}
