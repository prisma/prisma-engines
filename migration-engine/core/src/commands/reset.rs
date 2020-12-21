use crate::{api::MigrationApi, commands::command::MigrationCommand, CoreResult};
use migration_connector::MigrationConnector;

/// The `reset` command.
pub struct ResetCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for ResetCommand {
    type Input = ();
    type Output = ();

    async fn execute<C: MigrationConnector>(
        _input: &Self::Input,
        engine: &MigrationApi<C>,
    ) -> CoreResult<Self::Output> {
        tracing::debug!("Resetting the database.");

        engine.connector().reset().await?;

        Ok(())
    }
}
