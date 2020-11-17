use crate::migration_engine::MigrationEngine;
use crate::{commands::command::MigrationCommand, CoreResult};
use migration_connector::{DatabaseMigrationMarker, MigrationConnector};

/// The `reset` command.
pub struct ResetCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for ResetCommand {
    type Input = ();
    type Output = ();

    async fn execute<C, D>(_input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + 'static,
    {
        engine.connector().reset().await?;

        Ok(())
    }
}
