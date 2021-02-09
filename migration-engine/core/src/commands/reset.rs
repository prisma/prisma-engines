use crate::{commands::command::MigrationCommand, CoreResult};
use migration_connector::MigrationConnector;

/// The `reset` command.
pub struct ResetCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for ResetCommand {
    type Input = ();
    type Output = ();

    async fn execute<C: MigrationConnector>(_input: &Self::Input, connector: &C) -> CoreResult<Self::Output> {
        tracing::debug!("Resetting the database.");

        connector.reset().await?;

        Ok(())
    }
}
