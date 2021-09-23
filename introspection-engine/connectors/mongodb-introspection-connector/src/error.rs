use introspection_connector::ConnectorError;
use url::Url;
use user_facing_errors::{
    common::{DatabaseNotReachable, IncorrectDatabaseCredentials},
    KnownError,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, thiserror::Error)]
#[error("{kind}")]
pub struct Error {
    kind: Box<mongodb::error::ErrorKind>,
}

impl From<mongodb::error::Error> for Error {
    fn from(err: mongodb::error::Error) -> Self {
        Self { kind: err.kind }
    }
}

impl From<Error> for ConnectorError {
    fn from(err: Error) -> Self {
        let kind = match *err.kind {
            mongodb::error::ErrorKind::InvalidArgument { .. } => {
                introspection_connector::ErrorKind::QueryError(err.into())
            }
            mongodb::error::ErrorKind::Command(_) => introspection_connector::ErrorKind::QueryError(err.into()),
            mongodb::error::ErrorKind::Internal { .. } => todo!(),
            mongodb::error::ErrorKind::ConnectionPoolCleared { .. } => {
                introspection_connector::ErrorKind::QueryError(err.into())
            }
            mongodb::error::ErrorKind::InvalidResponse { .. } => {
                introspection_connector::ErrorKind::QueryError(err.into())
            }
            mongodb::error::ErrorKind::ServerSelection { .. } => {
                introspection_connector::ErrorKind::QueryError(err.into())
            }
            mongodb::error::ErrorKind::InvalidTlsConfig { .. } => introspection_connector::ErrorKind::TlsError {
                message: String::from("Failed to initialize a TLS connection."),
            },
            mongodb::error::ErrorKind::Write(_) => introspection_connector::ErrorKind::QueryError(err.into()),
            mongodb::error::ErrorKind::Transaction { .. } => introspection_connector::ErrorKind::QueryError(err.into()),
            mongodb::error::ErrorKind::IncompatibleServer { .. } => todo!(),
            _ => introspection_connector::ErrorKind::Generic(err.into()),
        };

        ConnectorError::from_kind(kind)
    }
}

pub(super) fn map_connection_errors(err: mongodb::error::Error, url: &Url) -> ConnectorError {
    match *err.kind {
        mongodb::error::ErrorKind::Authentication { .. } => {
            let known = KnownError::new(IncorrectDatabaseCredentials {
                database_user: url.username().into(),
                database_host: url.host_str().unwrap_or("(not available)").into(),
            });

            ConnectorError {
                user_facing_error: Some(known),
                kind: introspection_connector::ErrorKind::AuthenticationFailed {
                    user: url.username().into(),
                },
            }
        }
        mongodb::error::ErrorKind::DnsResolve { .. } => {
            let known = KnownError::new(DatabaseNotReachable {
                database_host: url.host_str().unwrap_or("(not available)").into(),
                database_port: url.port().unwrap_or(27019),
            });

            ConnectorError {
                user_facing_error: Some(known),
                kind: introspection_connector::ErrorKind::ConnectionError {
                    host: url.host_str().unwrap_or("(not available)").into(),
                    cause: err.into(),
                },
            }
        }
        _ => Error::from(err).into(),
    }
}
