use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use migration_connector::*;

/// Returns the version of the used db if available.
pub struct VersionCommand;

#[async_trait::async_trait]
impl MigrationCommand for VersionCommand {
    type Input = serde_json::Value;
    type Output = String;

    async fn execute<C, D>(_input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        Ok(connector.version())
    }
}
