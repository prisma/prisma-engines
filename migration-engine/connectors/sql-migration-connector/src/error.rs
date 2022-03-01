use migration_connector::ConnectorError;
use user_facing_errors::migration_engine::MigrateSystemDatabase;

#[derive(Debug)]
pub(crate) struct SystemDatabase(pub(crate) String);

impl From<SystemDatabase> for ConnectorError {
    fn from(err: SystemDatabase) -> ConnectorError {
        ConnectorError::user_facing(MigrateSystemDatabase { database_name: err.0 })
    }
}
