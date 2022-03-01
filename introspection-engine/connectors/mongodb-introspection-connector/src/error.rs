use introspection_connector::ConnectorError;
use mongodb_client::MongoConnectionString;
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

pub(super) fn map_connection_errors(err: mongodb::error::Error, conn_info: &MongoConnectionString) -> ConnectorError {
    match *err.kind {
        mongodb::error::ErrorKind::Authentication { .. } => {
            let known = KnownError::new(IncorrectDatabaseCredentials {
                database_user: conn_info.user.to_owned().unwrap_or_default(),
                database_host: conn_info.host_strings().join(","),
            });

            ConnectorError {
                user_facing_error: Some(known),
                kind: introspection_connector::ErrorKind::AuthenticationFailed {
                    user: conn_info.user.to_owned().unwrap_or_default(),
                },
            }
        }
        mongodb::error::ErrorKind::DnsResolve { .. } => {
            let host_port = conn_info.hosts.first().cloned();

            let known = KnownError::new(DatabaseNotReachable {
                database_host: host_port.as_ref().map(|hp| hp.0.to_owned()).unwrap_or_default(),
                database_port: host_port.as_ref().and_then(|hp| hp.1).unwrap_or(27019),
            });

            ConnectorError {
                user_facing_error: Some(known),
                kind: introspection_connector::ErrorKind::ConnectionError {
                    host: host_port.map(|hp| hp.0).unwrap_or_default(),
                    cause: err.into(),
                },
            }
        }
        _ => Error::from(err).into(),
    }
}
