use crate::MongoDbSchemaConnector;
use schema_connector::{BoxFuture, ConnectorResult, MigrationPersistence, Namespaces, SchemaFilter};

impl MigrationPersistence for MongoDbSchemaConnector {
    fn baseline_initialize(&mut self) -> schema_connector::BoxFuture<'_, ConnectorResult<()>> {
        unsupported_command_error()
    }

    fn initialize(
        &mut self,
        _namespaces: Option<Namespaces>,
        _filters: SchemaFilter,
    ) -> BoxFuture<'_, ConnectorResult<()>> {
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
            Result<Vec<schema_connector::MigrationRecord>, schema_connector::PersistenceNotInitializedError>,
        >,
    > {
        unsupported_command_error()
    }
}

fn unsupported_command_error<T: Send + Sync + 'static>() -> BoxFuture<'static, ConnectorResult<T>> {
    Box::pin(std::future::ready(Err(crate::unsupported_command_error())))
}
