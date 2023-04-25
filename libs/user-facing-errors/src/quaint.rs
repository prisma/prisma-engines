use crate::{common, query_engine, KnownError};
use common::ModelKind;
use indoc::formatdoc;
use quaint::{error::ErrorKind, prelude::ConnectionInfo};

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

pub fn invalid_connection_string_description(error_details: &str) -> String {
    let docs = r#"https://www.prisma.io/docs/reference/database-reference/connection-urls"#;

    let details = formatdoc! {r#"
            {} in database URL. Please refer to the documentation in {} for constructing a correct
            connection string. In some cases, certain characters must be escaped. Please
            check the string for any illegal characters."#, error_details, docs};

    details.replace('\n', " ")
}

pub fn render_quaint_error(kind: &ErrorKind, connection_info: &ConnectionInfo) -> Option<KnownError> {
    match (kind, connection_info) {
        #[cfg(feature = "sqlite")]
        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Sqlite { .. }) => {
            unreachable!(); // quaint implicitly creates sqlite databases
        }

        #[cfg(feature = "postgresql")]
        (ErrorKind::DatabaseDoesNotExist { db_name }, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseDoesNotExist::Postgres {
                database_name: db_name.to_string(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        #[cfg(feature = "mysql")]
        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseDoesNotExist::Mysql {
                database_name: url.dbname().to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }
        #[cfg(feature = "mssql")]
        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Mssql(url)) => {
            Some(KnownError::new(common::DatabaseDoesNotExist::Mssql {
                database_name: url.dbname().to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        #[cfg(feature = "postgresql")]
        (ErrorKind::DatabaseAccessDenied { .. }, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseAccessDenied {
                database_user: url.username().into_owned(),
                database_name: format!("{}.{}", url.dbname(), url.schema()),
            }))
        }

        #[cfg(feature = "mysql")]
        (ErrorKind::DatabaseAccessDenied { .. }, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseAccessDenied {
                database_user: url.username().into_owned(),
                database_name: url.dbname().to_owned(),
            }))
        }

        #[cfg(feature = "postgresql")]
        (ErrorKind::DatabaseAlreadyExists { db_name }, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseAlreadyExists {
                database_name: format!("{db_name}"),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        #[cfg(feature = "mysql")]
        (ErrorKind::DatabaseAlreadyExists { db_name }, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseAlreadyExists {
                database_name: format!("{db_name}"),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        #[cfg(feature = "postgresql")]
        (ErrorKind::AuthenticationFailed { user }, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::IncorrectDatabaseCredentials {
                database_user: format!("{user}"),
                database_host: url.host().to_owned(),
            }))
        }

        #[cfg(feature = "mysql")]
        (ErrorKind::AuthenticationFailed { user }, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::IncorrectDatabaseCredentials {
                database_user: format!("{user}"),
                database_host: url.host().to_owned(),
            }))
        }

        #[cfg(feature = "postgresql")]
        (ErrorKind::ConnectionError(_), ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseNotReachable {
                database_port: url.port(),
                database_host: url.host().to_owned(),
            }))
        }

        #[cfg(feature = "mysql")]
        (ErrorKind::ConnectionError(_), ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseNotReachable {
                database_port: url.port(),
                database_host: url.host().to_owned(),
            }))
        }

        (ErrorKind::UniqueConstraintViolation { constraint }, _) => {
            Some(KnownError::new(query_engine::UniqueKeyViolation {
                constraint: constraint.into(),
            }))
        }

        (ErrorKind::TlsError { message }, _) => Some(KnownError::new(common::TlsConnectionError {
            message: message.into(),
        })),

        #[cfg(feature = "mysql")]
        (ErrorKind::ConnectTimeout, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseNotReachable {
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        #[cfg(feature = "postgresql")]
        (ErrorKind::ConnectTimeout, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseNotReachable {
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        #[cfg(feature = "mssql")]
        (ErrorKind::ConnectTimeout, ConnectionInfo::Mssql(url)) => {
            Some(KnownError::new(common::DatabaseNotReachable {
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        #[cfg(feature = "mysql")]
        (ErrorKind::SocketTimeout, ConnectionInfo::Mysql(url)) => {
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

        #[cfg(feature = "postgresql")]
        (ErrorKind::SocketTimeout, ConnectionInfo::Postgres(url)) => {
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

        #[cfg(feature = "mssql")]
        (ErrorKind::SocketTimeout, ConnectionInfo::Mssql(url)) => {
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

        (ErrorKind::PoolTimeout { max_open, timeout, .. }, _) => Some(KnownError::new(query_engine::PoolTimeout {
            connection_limit: *max_open,
            timeout: *timeout,
        })),

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

        #[cfg(feature = "mysql")]
        (ErrorKind::TableDoesNotExist { table: model }, ConnectionInfo::Mysql(_)) => {
            Some(KnownError::new(common::InvalidModel {
                model: format!("{model}"),
                kind: ModelKind::Table,
            }))
        }

        #[cfg(feature = "postgresql")]
        (ErrorKind::TableDoesNotExist { table: model }, ConnectionInfo::Postgres(_)) => {
            Some(KnownError::new(common::InvalidModel {
                model: format!("{model}"),
                kind: ModelKind::Table,
            }))
        }

        #[cfg(feature = "sqlite")]
        (ErrorKind::TableDoesNotExist { table: model }, ConnectionInfo::Sqlite { .. }) => {
            Some(KnownError::new(common::InvalidModel {
                model: format!("{model}"),
                kind: ModelKind::Table,
            }))
        }

        #[cfg(feature = "mssql")]
        (ErrorKind::TableDoesNotExist { table: model }, ConnectionInfo::Mssql(_)) => {
            Some(KnownError::new(common::InvalidModel {
                model: format!("{model}"),
                kind: ModelKind::Table,
            }))
        }

        #[cfg(feature = "mssql")]
        (ErrorKind::IncorrectNumberOfParameters { expected, actual }, ConnectionInfo::Mssql(_)) => {
            Some(KnownError::new(common::IncorrectNumberOfParameters {
                expected: *expected,
                actual: *actual,
            }))
        }

        (ErrorKind::ConnectionClosed, _) => Some(KnownError::new(common::ConnectionClosed)),

        _ => None,
    }
}
