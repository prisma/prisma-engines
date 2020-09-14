use super::MigrationCommand;
use crate::migration_engine::MigrationEngine;
use serde::{Deserialize, Serialize};

/// Create and potentially apply a new migration.
pub struct CreateMigrationCommand;

/// The input to the `createMigration` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateMigrationInput {
    /// The filesystem path of the migrations directory to use.
    pub migrations_directory_path: String,
    /// The current prisma schema to use as a target for the generated migration.
    pub prisma_schema: String,
    /// If true, always generate a migration, but do not apply.
    pub draft: bool,
}

/// The output of the `createMigration` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateMigrationOutput {
    /// The number of executed migration steps, if a script was generated and executed. Otherwise 0.
    pub executed_steps: u32,
    /// The name of the newly generated migration directory, if any.
    pub generated_migration_name: Option<String>,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for CreateMigrationCommand {
    type Input = CreateMigrationInput;

    type Output = CreateMigrationOutput;

    async fn execute<C, D>(_input: &Self::Input, _engine: &MigrationEngine<C, D>) -> super::CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        todo!("createMigration command")
    }
}
