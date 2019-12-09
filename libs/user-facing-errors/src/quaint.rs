use crate::{common, query_engine, KnownError};
use quaint::{error::Error as QuaintError, prelude::ConnectionInfo};

pub fn render_quaint_error(quaint_error: &QuaintError, connection_info: &ConnectionInfo) -> Option<KnownError> {
    match (quaint_error, connection_info) {
        (QuaintError::DatabaseDoesNotExist { .. }, ConnectionInfo::Sqlite { file_path, .. }) => {
            KnownError::new(common::DatabaseDoesNotExist::Sqlite {
                database_file_path: file_path.clone(),
                database_file_name: std::path::Path::new(file_path)
                    .file_name()
                    .map(|osstr| osstr.to_string_lossy().into_owned())
                    .unwrap_or_else(|| file_path.clone()),
            })
            .ok()
        }

        (QuaintError::DatabaseDoesNotExist { .. }, ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseDoesNotExist::Postgres {
                database_name: url.dbname().to_owned(),
                database_schema_name: url.schema().to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (QuaintError::DatabaseDoesNotExist { .. }, ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::DatabaseDoesNotExist::Mysql {
                database_name: url.dbname().to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (QuaintError::DatabaseAccessDenied { .. }, ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseAccessDenied {
                database_user: url.username().into_owned(),
                database_name: format!("{}.{}", url.dbname(), url.schema()),
            })
            .ok()
        }

        (QuaintError::DatabaseAccessDenied { .. }, ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::DatabaseAccessDenied {
                database_user: url.username().into_owned(),
                database_name: url.dbname().to_owned(),
            })
            .ok()
        }

        (QuaintError::DatabaseAlreadyExists { db_name }, ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseAlreadyExists {
                database_name: db_name.to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (QuaintError::DatabaseAlreadyExists { db_name }, ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::DatabaseAlreadyExists {
                database_name: db_name.to_owned(),
                database_host: url.host().to_owned(),
                database_port: url.port(),
            })
            .ok()
        }

        (QuaintError::AuthenticationFailed { user }, ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::IncorrectDatabaseCredentials {
                database_user: user.to_owned(),
                database_host: url.host().to_owned(),
            })
            .ok()
        }

        (QuaintError::AuthenticationFailed { user }, ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::IncorrectDatabaseCredentials {
                database_user: user.to_owned(),
                database_host: url.host().to_owned(),
            })
            .ok()
        }

        (QuaintError::ConnectionError(_), ConnectionInfo::Postgres(url)) => {
            KnownError::new(common::DatabaseNotReachable {
                database_port: url.port(),
                database_host: url.host().to_owned(),
            })
            .ok()
        }

        (QuaintError::ConnectionError(_), ConnectionInfo::Mysql(url)) => {
            KnownError::new(common::DatabaseNotReachable {
                database_port: url.port(),
                database_host: url.host().to_owned(),
            })
            .ok()
        }

        (QuaintError::UniqueConstraintViolation { field_name }, _) => {
            KnownError::new(query_engine::UniqueKeyViolation {
                field_name: field_name.into(),
            })
            .ok()
        }

        (QuaintError::TlsError { message }, _) => KnownError::new(common::TlsConnectionError {
            message: message.into(),
        })
        .ok(),

        (QuaintError::ConnectTimeout, ConnectionInfo::Mysql(url)) => KnownError::new(common::DatabaseNotReachable {
            database_host: url.host().to_owned(),
            database_port: url.port(),
        })
        .ok(),

        (QuaintError::ConnectTimeout, ConnectionInfo::Postgres(url)) => KnownError::new(common::DatabaseNotReachable {
            database_host: url.host().to_owned(),
            database_port: url.port(),
        })
        .ok(),

        _ => None,
    }
}
