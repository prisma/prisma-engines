use super::MigrationCommand;
use crate::migration_engine::MigrationEngine;
use serde::{Deserialize, Serialize};

/// The input to the `DiagnoseMigrationHistory` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseMigrationHistoryInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
}

/// The output of the `DiagnoseMigrationHistory` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseMigrationHistoryOutput {}

/// Read the contents of the migrations directory and the migrations table, and
/// returns their relative statuses. At this stage, the migration engine only
/// reads, it does not write to the dev database nor the migrations directory.
pub struct DiagnoseMigrationHistoryCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for DiagnoseMigrationHistoryCommand {
    type Input = DiagnoseMigrationHistoryInput;

    type Output = DiagnoseMigrationHistoryOutput;

    async fn execute<C, D>(_input: &Self::Input, _engine: &MigrationEngine<C, D>) -> super::CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        todo!("diagnoseMigrationHistory command")
    }
}
