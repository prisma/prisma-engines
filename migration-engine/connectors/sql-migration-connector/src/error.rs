use migration_connector::{ConnectorError, ErrorKind};
use quaint::{
    error::{Error as QuaintError, ErrorKind as QuaintKind},
    prelude::ConnectionInfo,
};
use tracing_error::SpanTrace;
use user_facing_errors::{migration_engine::MigrateSystemDatabase, quaint::render_quaint_error};

pub(crate) fn quaint_error_to_connector_error(error: QuaintError, connection_info: &ConnectionInfo) -> ConnectorError {
    let user_facing_error = render_quaint_error(error.kind(), connection_info);

    let kind = match error.kind() {
        QuaintKind::DatabaseDoesNotExist { ref db_name } => ErrorKind::DatabaseDoesNotExist {
            db_name: db_name.clone(),
        },
        QuaintKind::DatabaseAlreadyExists { ref db_name } => ErrorKind::DatabaseAlreadyExists {
            db_name: db_name.clone(),
        },
        QuaintKind::DatabaseAccessDenied { ref db_name } => ErrorKind::DatabaseAccessDenied {
            database_name: db_name.clone(),
        },
        QuaintKind::AuthenticationFailed { ref user } => ErrorKind::AuthenticationFailed { user: user.clone() },
        QuaintKind::ConnectTimeout(..) => ErrorKind::ConnectTimeout,
        QuaintKind::ConnectionError { .. } => ErrorKind::ConnectionError {
            cause: error.into(),
            host: connection_info.host().to_owned(),
        },
        QuaintKind::Timeout(..) => ErrorKind::Timeout,
        QuaintKind::TlsError { message } => ErrorKind::TlsError {
            message: message.clone(),
        },
        _ => ErrorKind::QueryError(error.into()),
    };

    ConnectorError {
        user_facing_error,
        kind,
        context: SpanTrace::capture(),
    }
}

pub(crate) type CheckDatabaseInfoResult = Result<(), SystemDatabase>;

#[derive(Debug)]
pub(crate) struct SystemDatabase(pub(crate) String);

impl From<SystemDatabase> for ConnectorError {
    fn from(err: SystemDatabase) -> ConnectorError {
        ConnectorError::user_facing_error(MigrateSystemDatabase {
            database_name: err.0.clone(),
        })
    }
}
