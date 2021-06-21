use crate::MongoDbMigrationConnector;
use migration_connector::{ConnectorResult, DestructiveChangeChecker, DestructiveChangeDiagnostics, Migration};

#[async_trait::async_trait]
impl DestructiveChangeChecker for MongoDbMigrationConnector {
    async fn check(&self, _database_migration: &Migration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        Ok(DestructiveChangeDiagnostics::new())
    }

    fn pure_check(&self, _database_migration: &Migration) -> DestructiveChangeDiagnostics {
        DestructiveChangeDiagnostics::new()
    }
}
