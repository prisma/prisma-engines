use crate::{common, query_engine, KnownError};
use indoc::formatdoc;
use quaint::{error::ErrorKind, prelude::ConnectionInfo};

#[cfg(not(target_arch = "wasm32"))]
use quaint::{connector::NativeConnectionInfo, error::NativeErrorKind};

impl From<&quaint::error::DatabaseConstraint> for query_engine::DatabaseConstraint {
    fn from(other: &quaint::error::DatabaseConstraint) -> Self {
        match other {
            quaint::error::DatabaseConstraint::Fields(fields) => Self::Fields(fields.to_vec()),
            quaint::error::DatabaseConstraint::Index(index) => Self::Index(index.to_string()),
            quaint::error::DatabaseConstraint::ForeignKey => Self::ForeignKey,
            quaint::error::DatabaseConstraint::CannotParse => Self::CannotParse,
        }
    }
}

impl From<quaint::error::DatabaseConstraint> for query_engine::DatabaseConstraint {
    fn from(other: quaint::error::DatabaseConstraint) -> Self {
        match other {
            quaint::error::DatabaseConstraint::Fields(fields) => Self::Fields(fields.to_vec()),
            quaint::error::DatabaseConstraint::Index(index) => Self::Index(index),
            quaint::error::DatabaseConstraint::ForeignKey => Self::ForeignKey,
            quaint::error::DatabaseConstraint::CannotParse => Self::CannotParse,
        }
    }
}

pub fn render_quaint_error(kind: &ErrorKind, connection_info: &ConnectionInfo) -> Option<KnownError> {
    let default_value: Option<KnownError> = None;

    match (kind, connection_info) {
        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::External(_)) => default_value,
        #[cfg(not(target_arch = "wasm32"))]
        (ErrorKind::DatabaseDoesNotExist { db_name }, _) => match connection_info {
            ConnectionInfo::Native(NativeConnectionInfo::Postgres(url)) => {
                Some(KnownError::new(common::DatabaseDoesNotExist::Postgres {
                    database_name: db_name.to_string(),
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            ConnectionInfo::Native(NativeConnectionInfo::Mysql(url)) => {
                Some(KnownError::new(common::DatabaseDoesNotExist::Mysql {
                    database_name: url.dbname().to_owned(),
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            ConnectionInfo::Native(NativeConnectionInfo::Mssql(url)) => {
                Some(KnownError::new(common::DatabaseDoesNotExist::Mssql {
                    database_name: url.dbname().to_owned(),
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            _ => unreachable!(), // quaint implicitly creates sqlite databases
        },

        (ErrorKind::DatabaseAccessDenied { .. }, ConnectionInfo::External(_)) => default_value,
        #[cfg(not(target_arch = "wasm32"))]
        (ErrorKind::DatabaseAccessDenied { .. }, _) => match connection_info {
            ConnectionInfo::Native(NativeConnectionInfo::Postgres(url)) => {
                Some(KnownError::new(common::DatabaseAccessDenied {
                    database_user: url.username().into_owned(),
                    database_name: format!("{}.{}", url.dbname(), url.schema()),
                }))
            }
            ConnectionInfo::Native(NativeConnectionInfo::Mysql(url)) => {
                Some(KnownError::new(common::DatabaseAccessDenied {
                    database_user: url.username().into_owned(),
                    database_name: url.dbname().to_owned(),
                }))
            }
            _ => unreachable!(),
        },

        (ErrorKind::DatabaseAlreadyExists { .. }, ConnectionInfo::External(_)) => default_value,
        #[cfg(not(target_arch = "wasm32"))]
        (ErrorKind::DatabaseAlreadyExists { db_name }, _) => match connection_info {
            ConnectionInfo::Native(NativeConnectionInfo::Postgres(url)) => {
                Some(KnownError::new(common::DatabaseAlreadyExists {
                    database_name: format!("{db_name}"),
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            ConnectionInfo::Native(NativeConnectionInfo::Mysql(url)) => {
                Some(KnownError::new(common::DatabaseAlreadyExists {
                    database_name: format!("{db_name}"),
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            _ => unreachable!(),
        },

        (ErrorKind::AuthenticationFailed { .. }, ConnectionInfo::External(_)) => default_value,
        #[cfg(not(target_arch = "wasm32"))]
        (ErrorKind::AuthenticationFailed { user }, _) => match connection_info {
            ConnectionInfo::Native(NativeConnectionInfo::Postgres(url)) => {
                Some(KnownError::new(common::IncorrectDatabaseCredentials {
                    database_user: format!("{user}"),
                    database_host: url.host().to_owned(),
                }))
            }
            ConnectionInfo::Native(NativeConnectionInfo::Mysql(url)) => {
                Some(KnownError::new(common::IncorrectDatabaseCredentials {
                    database_user: format!("{user}"),
                    database_host: url.host().to_owned(),
                }))
            }
            _ => unreachable!(),
        },

        (ErrorKind::SocketTimeout { .. }, ConnectionInfo::External(_)) => default_value,
        #[cfg(not(target_arch = "wasm32"))]
        (ErrorKind::SocketTimeout, _) => match connection_info {
            ConnectionInfo::Native(NativeConnectionInfo::Postgres(url)) => {
                let time = match url.socket_timeout() {
                    Some(dur) => format!("{}s", dur.as_secs()),
                    None => String::from("N/A"),
                };

                Some(KnownError::new(common::DatabaseOperationTimeout {
                    time,
                    context: "Socket timeout (the database failed to respond to a query within the configured timeout — see https://pris.ly/d/mssql-connector for more details.)."
                        .into(),
                }))
            }
            ConnectionInfo::Native(NativeConnectionInfo::Mysql(url)) => {
                let time = match url.socket_timeout() {
                    Some(dur) => format!("{}s", dur.as_secs()),
                    None => String::from("N/A"),
                };

                Some(KnownError::new(common::DatabaseOperationTimeout {
                    time,
                    context: "Socket timeout (the database failed to respond to a query within the configured timeout — see https://pris.ly/d/mysql-connector for more details.)."
                        .into(),
                }))
            }
            ConnectionInfo::Native(NativeConnectionInfo::Mssql(url)) => {
                let time = match url.socket_timeout() {
                    Some(dur) => format!("{}s", dur.as_secs()),
                    None => String::from("N/A"),
                };

                Some(KnownError::new(common::DatabaseOperationTimeout {
                    time,
                    context: "Socket timeout (the database failed to respond to a query within the configured timeout — see https://pris.ly/d/postgres-connector for more details.)."
                        .into(),
                }))
            }
            _ => unreachable!(),
        },

        (ErrorKind::TableDoesNotExist { .. }, ConnectionInfo::External(_)) => default_value,
        #[cfg(not(target_arch = "wasm32"))]
        (ErrorKind::TableDoesNotExist { table: model }, _) => match connection_info {
            ConnectionInfo::Native(NativeConnectionInfo::Postgres(_)) => Some(KnownError::new(common::InvalidModel {
                model: format!("{model}"),
                kind: common::ModelKind::Table,
            })),
            ConnectionInfo::Native(NativeConnectionInfo::Mysql(_)) => Some(KnownError::new(common::InvalidModel {
                model: format!("{model}"),
                kind: common::ModelKind::Table,
            })),
            ConnectionInfo::Native(NativeConnectionInfo::Sqlite { .. }) => {
                Some(KnownError::new(common::InvalidModel {
                    model: format!("{model}"),
                    kind: common::ModelKind::Table,
                }))
            }
            ConnectionInfo::Native(NativeConnectionInfo::Mssql(_)) => Some(KnownError::new(common::InvalidModel {
                model: format!("{model}"),
                kind: common::ModelKind::Table,
            })),
            _ => unreachable!(),
        },

        (ErrorKind::UniqueConstraintViolation { constraint }, _) => {
            Some(KnownError::new(query_engine::UniqueKeyViolation {
                constraint: constraint.into(),
            }))
        }

        (ErrorKind::DatabaseUrlIsInvalid(details), _connection_info) => {
            Some(KnownError::new(common::InvalidConnectionString {
                details: details.to_owned(),
            }))
        }

        (ErrorKind::LengthMismatch { column }, _connection_info) => {
            Some(KnownError::new(query_engine::InputValueTooLong {
                column_name: format!("{column}"),
            }))
        }

        (ErrorKind::ValueOutOfRange { message }, _connection_info) => {
            Some(KnownError::new(query_engine::ValueOutOfRange {
                details: message.clone(),
            }))
        }

        #[cfg(not(target_arch = "wasm32"))]
        (ErrorKind::Native(native_error_kind), _) => match (native_error_kind, connection_info) {
            (NativeErrorKind::ConnectionError(_), ConnectionInfo::Native(NativeConnectionInfo::Postgres(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_port: url.port(),
                    database_host: url.host().to_owned(),
                }))
            }
            (NativeErrorKind::ConnectionError(_), ConnectionInfo::Native(NativeConnectionInfo::Mysql(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_port: url.port(),
                    database_host: url.host().to_owned(),
                }))
            }
            (NativeErrorKind::ConnectionError(_), ConnectionInfo::Native(NativeConnectionInfo::Mssql(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_port: url.port(),
                    database_host: url.host().to_owned(),
                }))
            }
            (NativeErrorKind::TlsError { message }, _) => Some(KnownError::new(common::TlsConnectionError {
                message: message.into(),
            })),
            (NativeErrorKind::ConnectTimeout, ConnectionInfo::Native(NativeConnectionInfo::Postgres(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            (NativeErrorKind::ConnectTimeout, ConnectionInfo::Native(NativeConnectionInfo::Mysql(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            (NativeErrorKind::ConnectTimeout, ConnectionInfo::Native(NativeConnectionInfo::Mssql(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            (NativeErrorKind::PoolTimeout { max_open, timeout, .. }, _) => {
                Some(KnownError::new(query_engine::PoolTimeout {
                    connection_limit: *max_open,
                    timeout: *timeout,
                }))
            }
            (NativeErrorKind::ConnectionClosed, _) => Some(KnownError::new(common::ConnectionClosed)),
            _ => unreachable!(),
        },

        _ => None,
    }
}
