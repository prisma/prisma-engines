use super::MigrationCommand;
use crate::migration_engine::MigrationEngine;
use serde::{Deserialize, Serialize};

/// The input to the `ApplyMigrations` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMigrationsInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
}

/// The output of the `ApplyMigrations` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMigrationsOutput {
    /// The names of the migrations that were just applied. Empty if no migration was applied.
    pub applied_migration_names: Vec<String>,
}

/// Read the contents of the migrations directory and the migrations table, and
/// returns their relative statuses. At this stage, the migration engine only
/// reads, it does not write to the dev database nor the migrations directory.
pub struct ApplyMigrationsCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for ApplyMigrationsCommand {
    type Input = ApplyMigrationsInput;

    type Output = ApplyMigrationsOutput;

    async fn execute<C, D>(_input: &Self::Input, _engine: &MigrationEngine<C, D>) -> super::CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        todo!("ApplyMigrations command")
    }
}
