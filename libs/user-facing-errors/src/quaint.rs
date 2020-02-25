use crate::{common, query_engine, KnownError};
use quaint::{error::ErrorKind, prelude::ConnectionInfo};

impl From<&quaint::error::DatabaseConstraint> for crate::query_engine::DatabaseConstraint {
    fn from(other: &quaint::error::DatabaseConstraint) -> Self {
        match other {
            quaint::error::DatabaseConstraint::Fields(fields) => Self::Fields(fields.to_vec()),
            quaint::error::DatabaseConstraint::Index(index) => Self::Index(index.to_string()),
            quaint::error::DatabaseConstraint::ForeignKey => Self::ForeignKey,
        }
    }
}

impl From<quaint::error::DatabaseConstraint> for crate::query_engine::DatabaseConstraint {
    fn from(other: quaint::error::DatabaseConstraint) -> Self {
        match other {
            quaint::error::DatabaseConstraint::Fields(fields) => Self::Fields(fields.to_vec()),
            quaint::error::DatabaseConstraint::Index(index) => Self::Index(index.to_string()),
            quaint::error::DatabaseConstraint::ForeignKey => Self::ForeignKey,
        }
    }
}

pub fn render_quaint_error(kind: &ErrorKind, connection_info: &ConnectionInfo) -> Option<KnownError> {
    match (kind, connection_info) {
        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Sqlite { file_path, .. }) => {
            KnownError::new(common::DatabaseDoesNotExist::Sqlite {
                database_file_path: file_path.clone(),
                database_file_name: std::path::Path::new(file_path)
                    .file_name()
                    .map(|osstr| osstr.to_string_lossy().into_owned())
                    .unwrap_or_else(|| file_path.clone()),
            })
            .ok()
        }

        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseDoesNotExist::Postgres {
                database_name: url.dbname().to_owned(),
                database_schema_name: url.schema().to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::DatabaseDoesNotExist::Mysql {
                database_name: url.dbname().to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (ErrorKind::DatabaseAccessDenied { .. }, ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseAccessDenied {
                database_user: url.username().into_owned(),
                database_name: format!("{}.{}", url.dbname(), url.schema()),
            })
            .ok()
        }

        (ErrorKind::DatabaseAccessDenied { .. }, ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::DatabaseAccessDenied {
                database_user: url.username().into_owned(),
                database_name: url.dbname().to_owned(),
            })
            .ok()
        }

        (ErrorKind::DatabaseAlreadyExists { db_name }, ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseAlreadyExists {
                database_name: db_name.to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (ErrorKind::DatabaseAlreadyExists { db_name }, ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::DatabaseAlreadyExists {
                database_name: db_name.to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (ErrorKind::AuthenticationFailed { user }, ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::IncorrectDatabaseCredentials {
                database_user: user.to_owned(),
                database_host: url.host().to_owned(),
            })
            .ok()
        }

        (ErrorKind::AuthenticationFailed { user }, ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::IncorrectDatabaseCredentials {
                database_user: user.to_owned(),
                database_host: url.host().to_owned(),
            })
            .ok()
        }

        (ErrorKind::ConnectionError(_), ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseNotReachable {
                database_port: url.port(),
                database_host: url.host().to_owned(),
            })
            .ok()
        }

        (ErrorKind::ConnectionError(_), ConnectionInfo::Mysql(url)) => KnownError::new(common::DatabaseNotReachable {
            database_port: url.port(),
            database_host: url.host().to_owned(),
        })
        .ok(),

        (ErrorKind::UniqueConstraintViolation { constraint }, _) => KnownError::new(query_engine::UniqueKeyViolation {
            constraint: constraint.into(),
        })
        .ok(),

        (ErrorKind::TlsError { message }, _) => KnownError::new(common::TlsConnectionError {
            message: message.into(),
        })
        .ok(),

        (ErrorKind::ConnectTimeout(..), ConnectionInfo::Mysql(url)) => KnownError::new(common::DatabaseNotReachable {
            database_host: url.host().to_owned(),
            database_port: url.port(),
        })
        .ok(),

        (ErrorKind::ConnectTimeout(..), ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseNotReachable {
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (ErrorKind::DatabaseUrlIsInvalid(details), _connection_info) => {
            KnownError::new(common::InvalidDatabaseString {
                details: details.to_owned(),
            })
            .ok()
        }

        _ => None,
    }
}
