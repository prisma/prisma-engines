use migration_connector::ConnectorError;
use quaint::{error::Error as QuaintError, prelude::ConnectionInfo};
use user_facing_errors::{migration_engine::MigrateSystemDatabase, quaint::render_quaint_error};

pub(crate) fn quaint_error_to_connector_error(error: QuaintError, connection_info: &ConnectionInfo) -> ConnectorError {
    match render_quaint_error(error.kind(), connection_info) {
        Some(user_facing_error) => user_facing_error.into(),
        None => ConnectorError::generic(anyhow::Error::new(error).context("Database error")),
    }
}

#[derive(Debug)]
pub(crate) struct SystemDatabase(pub(crate) String);

impl From<SystemDatabase> for ConnectorError {
    fn from(err: SystemDatabase) -> ConnectorError {
        ConnectorError::user_facing_error(MigrateSystemDatabase { database_name: err.0 })
    }
}
