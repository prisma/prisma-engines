use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use migration_connector::*;

/// The `reset` command.
pub struct ResetCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for ResetCommand {
    type Input = ();
    type Output = ();

    async fn execute<C, D>(_input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + 'static,
    {
        engine.reset().await?;

        Ok(Default::default())
    }
}
