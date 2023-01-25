use crate::MongoDbMigrationConnector;
use migration_connector::{BoxFuture, ConnectorResult, MigrationPersistence, Namespaces};

impl MigrationPersistence for MongoDbMigrationConnector {
    fn baseline_initialize(&mut self) -> migration_connector::BoxFuture<'_, ConnectorResult<()>> {
        unsupported_command_error()
    }

    fn initialize(&mut self, _namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        unsupported_command_error()
    }

    fn mark_migration_applied_impl(
        &mut self,
        _migration_name: &str,
        _checksum: &str,
    ) -> BoxFuture<'_, ConnectorResult<String>> {
        unsupported_command_error()
    }

    fn mark_migration_rolled_back_by_id(&mut self, _migration_id: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        unsupported_command_error()
    }

    fn record_migration_started_impl(
        &mut self,
        _migration_name: &str,
        _checksum: &str,
    ) -> BoxFuture<'_, ConnectorResult<String>> {
        unsupported_command_error()
    }

    fn record_successful_step(&mut self, _id: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        unsupported_command_error()
    }

    fn record_failed_step(&mut self, _id: &str, _logs: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        unsupported_command_error()
    }

    fn record_migration_finished(&mut self, _id: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        unsupported_command_error()
    }

    fn list_migrations(
        &mut self,
    ) -> BoxFuture<
        '_,
        ConnectorResult<
            Result<Vec<migration_connector::MigrationRecord>, migration_connector::PersistenceNotInitializedError>,
        >,
    > {
        unsupported_command_error()
    }
}

fn unsupported_command_error<T: Send + Sync + 'static>() -> BoxFuture<'static, ConnectorResult<T>> {
    Box::pin(std::future::ready(Err(crate::unsupported_command_error())))
}
