use crate::{common, query_engine, KnownError};
use common::ModelKind;
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
            quaint::error::DatabaseConstraint::Index(index) => Self::Index(index),
            quaint::error::DatabaseConstraint::ForeignKey => Self::ForeignKey,
        }
    }
}

pub fn render_quaint_error(kind: &ErrorKind, connection_info: &ConnectionInfo) -> Option<KnownError> {
    match (kind, connection_info) {
        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Sqlite { file_path, .. }) => {
            Some(KnownError::new(common::DatabaseDoesNotExist::Sqlite {
                database_file_path: file_path.clone(),
                database_file_name: std::path::Path::new(file_path)
                    .file_name()
                    .map(|osstr| osstr.to_string_lossy().into_owned())
                    .unwrap_or_else(|| file_path.clone()),
            }))
        }

        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseDoesNotExist::Postgres {
                database_name: url.dbname().to_owned(),
                database_schema_name: url.schema().to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        (ErrorKind::DatabaseDoesNotExist { .. }, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseDoesNotExist::Mysql {
                database_name: url.dbname().to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        (ErrorKind::DatabaseAccessDenied { .. }, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseAccessDenied {
                database_user: url.username().into_owned(),
                database_name: format!("{}.{}", url.dbname(), url.schema()),
            }))
        }

        (ErrorKind::DatabaseAccessDenied { .. }, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseAccessDenied {
                database_user: url.username().into_owned(),
                database_name: url.dbname().to_owned(),
            }))
        }

        (ErrorKind::DatabaseAlreadyExists { db_name }, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseAlreadyExists {
                database_name: db_name.to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        (ErrorKind::DatabaseAlreadyExists { db_name }, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseAlreadyExists {
                database_name: db_name.to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        (ErrorKind::AuthenticationFailed { user }, ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::IncorrectDatabaseCredentials {
                database_user: user.to_owned(),
                database_host: url.host().to_owned(),
            }))
        }

        (ErrorKind::AuthenticationFailed { user }, ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::IncorrectDatabaseCredentials {
                database_user: user.to_owned(),
                database_host: url.host().to_owned(),
            }))
        }

        (ErrorKind::ConnectionError(_), ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseNotReachable {
                database_port: url.port(),
                database_host: url.host().to_owned(),
            }))
        }

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

        (ErrorKind::ConnectTimeout(..), ConnectionInfo::Mysql(url)) => {
            Some(KnownError::new(common::DatabaseNotReachable {
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        (ErrorKind::ConnectTimeout(..), ConnectionInfo::Postgres(url)) => {
            Some(KnownError::new(common::DatabaseNotReachable {
                database_host: url.host().to_owned(),
                database_port: url.port(),
            }))
        }

        (ErrorKind::DatabaseUrlIsInvalid(details), _connection_info) => {
            Some(KnownError::new(common::InvalidDatabaseString {
                details: details.to_owned(),
            }))
        }

        (ErrorKind::LengthMismatch { column }, _connection_info) => {
            Some(KnownError::new(query_engine::InputValueTooLong {
                column_name: column.clone().unwrap_or_else(|| "<unknown>".to_string()),
            }))
        }

        (ErrorKind::ValueOutOfRange { message }, _connection_info) => {
            Some(KnownError::new(query_engine::ValueOutOfRange {
                details: message.clone(),
            }))
        }

        (ErrorKind::TableDoesNotExist { table: model }, ConnectionInfo::Mysql(_)) => {
            Some(KnownError::new(common::InvalidModel {
                model: model.into(),
                kind: ModelKind::Table,
            }))
        }

        (ErrorKind::TableDoesNotExist { table: model }, ConnectionInfo::Postgres(_)) => {
            Some(KnownError::new(common::InvalidModel {
                model: model.into(),
                kind: ModelKind::Table,
            }))
        }

        (ErrorKind::TableDoesNotExist { table: model }, ConnectionInfo::Sqlite { .. }) => {
            Some(KnownError::new(common::InvalidModel {
                model: model.into(),
                kind: ModelKind::Table,
            }))
        }

        (ErrorKind::TableDoesNotExist { table: model }, ConnectionInfo::Mssql(_)) => {
            Some(KnownError::new(common::InvalidModel {
                model: model.into(),
                kind: ModelKind::Table,
            }))
        }

        _ => None,
    }
}
