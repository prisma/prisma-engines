use super::MigrationCommand;
use crate::migration_engine::MigrationEngine;
use serde::Deserialize;

/// Input to the `Initialize` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitializeInput {
    /// Path to the migrations directory.
    pub migrations_directory_path: String,
}

/// Output of the `Initialize` command.
pub type InitializeOutput = ();

/// Initialize the migrations directory and the migrations table.
pub struct InitializeCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for InitializeCommand {
    type Input = InitializeInput;

    type Output = InitializeOutput;

    async fn execute<C, D>(_input: &Self::Input, _engine: &MigrationEngine<C, D>) -> super::CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        todo!("initialize command")
    }
}
