use crate::MongoDbMigrationConnector;
use migration_connector::{
    BoxFuture, ConnectorResult, DestructiveChangeChecker, DestructiveChangeDiagnostics, Migration,
};

impl DestructiveChangeChecker for MongoDbMigrationConnector {
    fn check<'a>(
        &'a mut self,
        _database_migration: &'a Migration,
    ) -> BoxFuture<'a, ConnectorResult<DestructiveChangeDiagnostics>> {
        Box::pin(std::future::ready(Ok(DestructiveChangeDiagnostics::new())))
    }

    fn pure_check(&self, _database_migration: &Migration) -> DestructiveChangeDiagnostics {
        DestructiveChangeDiagnostics::new()
    }
}
