use crate::{
    connector::mysql::error::MysqlError,
    error::{Error, ErrorKind},
};
use mysql_async as my;

impl From<&my::ServerError> for MysqlError {
    fn from(value: &my::ServerError) -> Self {
        MysqlError {
            code: value.code,
            message: value.message.to_owned(),
            state: value.state.to_owned(),
        }
    }
}

impl From<my::Error> for Error {
    fn from(e: my::Error) -> Error {
        match e {
            my::Error::Io(my::IoError::Tls(err)) => Error::builder(ErrorKind::TlsError {
                message: err.to_string(),
            })
            .build(),
            my::Error::Io(my::IoError::Io(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                Error::builder(ErrorKind::ConnectionClosed).build()
            }
            my::Error::Io(io_error) => Error::builder(ErrorKind::ConnectionError(io_error.into())).build(),
            my::Error::Driver(e) => Error::builder(ErrorKind::QueryError(e.into())).build(),
            my::Error::Server(ref server_error) => {
                let mysql_error: MysqlError = server_error.into();
                mysql_error.into()
            }
            e => Error::builder(ErrorKind::QueryError(e.into())).build(),
        }
    }
}
