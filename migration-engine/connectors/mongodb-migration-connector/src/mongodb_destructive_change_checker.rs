use migration_connector::{ConnectorResult, DestructiveChangeChecker, DestructiveChangeDiagnostics};

use crate::{mongodb_migration::MongoDbMigration, MongoDbMigrationConnector};

#[async_trait::async_trait]
impl DestructiveChangeChecker<MongoDbMigration> for MongoDbMigrationConnector {
    async fn check(&self, _database_migration: &MongoDbMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        Ok(DestructiveChangeDiagnostics::new())
    }

    fn pure_check(&self, _database_migration: &MongoDbMigration) -> DestructiveChangeDiagnostics {
        DestructiveChangeDiagnostics::new()
    }
}
