use crate::error::{Error, DatabaseConstraint};
use mysql_async as my;
use std::io::ErrorKind as IoErrorKind;

impl From<my::error::Error> for Error {
    fn from(e: my::error::Error) -> Error {
        use my::error::ServerError;

        match e {
            my::error::Error::Io(io_error) => match io_error.kind() {
                IoErrorKind::ConnectionRefused => Error::ConnectionError(io_error.into()),
                _ => Error::QueryError(io_error.into()),
            },
            my::error::Error::Driver(e) => Error::QueryError(e.into()),
            my::error::Error::Server(ServerError { ref message, code, .. }) if code == 1062 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().map(|s| s.split('\'').collect()).unwrap();

                let index = splitted[1].split(".").last().unwrap().to_string();

                Error::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Index(index),
                }
            }
            my::error::Error::Server(ServerError { ref message, code, .. }) if code == 1263 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().map(|s| s.split('\'').collect()).unwrap();

                Error::NullConstraintViolation {
                    constraint: DatabaseConstraint::Index(splitted[1].to_string()),
                }
            }
            my::error::Error::Server(ServerError { ref message, code, .. }) if code == 1364 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.get(1).map(|s| s.split('\'').collect()).unwrap();

                Error::NullConstraintViolation {
                    constraint: DatabaseConstraint::Fields(vec![splitted[1].to_string()]),
                }
            }
            my::error::Error::Server(ServerError { ref message, code, .. }) if code == 1049 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().map(|s| s.split('\'').collect()).unwrap();
                let db_name: String = splitted[1].into();

                Error::DatabaseDoesNotExist { db_name }
            }
            my::error::Error::Server(ServerError { ref message, code, .. }) if code == 1007 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted[3].split('\'').collect();
                let db_name: String = splitted[1].into();

                Error::DatabaseAlreadyExists { db_name }
            }
            my::error::Error::Server(ServerError { ref message, code, .. }) if code == 1044 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().map(|s| s.split('\'').collect()).unwrap();
                let db_name: String = splitted[1].into();

                Error::DatabaseAccessDenied { db_name }
            }
            my::error::Error::Server(ServerError { ref message, code, .. }) if code == 1045 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted[4].split('@').collect();
                let splitted: Vec<&str> = splitted[0].split('\'').collect();
                let user: String = splitted[1].into();

                Error::AuthenticationFailed { user }
            }
            e => Error::QueryError(e.into()),
        }
    }
}
