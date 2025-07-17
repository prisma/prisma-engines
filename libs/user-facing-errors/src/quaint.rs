use crate::{KnownError, common, query_engine};
use indoc::formatdoc;
use quaint::connector::NativeConnectionInfo;
use quaint::error::ErrorKind;

#[cfg(any(
    feature = "mssql-native",
    feature = "mysql-native",
    feature = "postgresql-native",
    feature = "sqlite-native"
))]
use quaint::error::NativeErrorKind;

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

pub fn render_quaint_error(kind: &ErrorKind, connection_info: Option<&NativeConnectionInfo>) -> Option<KnownError> {
    match kind {
        ErrorKind::DatabaseDoesNotExist { db_name } => Some(KnownError::new(common::DatabaseDoesNotExist {
            database_name: db_name.to_string(),
        })),

        ErrorKind::DatabaseAccessDenied { db_name } => Some(KnownError::new(common::DatabaseAccessDenied {
            database_name: db_name.to_string(),
        })),

        ErrorKind::DatabaseAlreadyExists { db_name } => Some(KnownError::new(common::DatabaseAlreadyExists {
            database_name: db_name.to_string(),
        })),

        ErrorKind::AuthenticationFailed { user } => Some(KnownError::new(common::IncorrectDatabaseCredentials {
            database_user: user.to_string(),
        })),

        ErrorKind::SocketTimeout => {
            let extra_hint = match connection_info {
                #[cfg(feature = "postgresql-native")]
                Some(NativeConnectionInfo::Postgres(_)) => {
                    "— see https://pris.ly/d/postgresql-connector for more details"
                }
                #[cfg(feature = "mysql-native")]
                Some(NativeConnectionInfo::Mysql(_)) => "— see https://pris.ly/d/mysql-connector for more details",
                #[cfg(feature = "mssql-native")]
                Some(NativeConnectionInfo::Mssql(_)) => "— see https://pris.ly/d/mssql-connector for more details",
                #[cfg(feature = "sqlite-native")]
                Some(NativeConnectionInfo::Sqlite { .. } | NativeConnectionInfo::InMemorySqlite { .. }) => {
                    "— see https://pris.ly/d/sqlite-connector for more details"
                }
                _ => "",
            };

            Some(KnownError::new(common::DatabaseOperationTimeout {
                extra_hint: extra_hint.into(),
            }))
        }
        ErrorKind::TableDoesNotExist { table: model } => Some(KnownError::new(common::InvalidModel {
            model: format!("{model}"),
            kind: common::ModelKind::Table,
        })),
        ErrorKind::UniqueConstraintViolation { constraint } => {
            Some(KnownError::new(query_engine::UniqueKeyViolation {
                constraint: constraint.into(),
            }))
        }

        ErrorKind::DatabaseUrlIsInvalid(details) => Some(KnownError::new(common::InvalidConnectionString {
            details: details.to_owned(),
        })),

        ErrorKind::LengthMismatch { column } => Some(KnownError::new(query_engine::InputValueTooLong {
            column_name: format!("{column}"),
        })),

        ErrorKind::ValueOutOfRange { message } => Some(KnownError::new(query_engine::ValueOutOfRange {
            details: message.clone(),
        })),

        #[cfg(any(
            feature = "mssql-native",
            feature = "mysql-native",
            feature = "postgresql-native",
            feature = "sqlite-native"
        ))]
        ErrorKind::Native(native_error_kind) => match (native_error_kind, connection_info) {
            #[cfg(feature = "postgresql-native")]
            (NativeErrorKind::ConnectionError(_), Some(NativeConnectionInfo::Postgres(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_port: url.port(),
                    database_host: url.host().to_owned(),
                }))
            }
            #[cfg(feature = "mysql-native")]
            (NativeErrorKind::ConnectionError(_), Some(NativeConnectionInfo::Mysql(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_port: url.port(),
                    database_host: url.host().to_owned(),
                }))
            }
            #[cfg(feature = "mssql-native")]
            (NativeErrorKind::ConnectionError(_), Some(NativeConnectionInfo::Mssql(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_port: url.port(),
                    database_host: url.host().to_owned(),
                }))
            }
            (NativeErrorKind::TlsError { message }, _) => Some(KnownError::new(common::TlsConnectionError {
                message: message.into(),
            })),
            #[cfg(feature = "postgresql-native")]
            (NativeErrorKind::ConnectTimeout, Some(NativeConnectionInfo::Postgres(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            #[cfg(feature = "mysql-native")]
            (NativeErrorKind::ConnectTimeout, Some(NativeConnectionInfo::Mysql(url))) => {
                Some(KnownError::new(common::DatabaseNotReachable {
                    database_host: url.host().to_owned(),
                    database_port: url.port(),
                }))
            }
            #[cfg(feature = "mssql-native")]
            (NativeErrorKind::ConnectTimeout, Some(NativeConnectionInfo::Mssql(url))) => {
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
