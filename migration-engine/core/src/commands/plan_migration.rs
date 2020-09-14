use super::MigrationCommand;
use crate::migration_engine::MigrationEngine;
use serde::{Deserialize, Serialize};

/// The input to the `planMigration` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlanMigrationInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
    /// The prisma schema to migrate to.
    pub prisma_schema: String,
}

/// The output of the `planMigration` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlanMigrationOutput {
    /// Proposed action to reconcile migrations folder history and local database schema.
    pub proposed_action: Option<String>,
    /// The number of migration steps we would generate.
    pub migrations_step: u32,
    /// Destructive change warnings.
    pub warnings: Vec<String>,
    /// Steps that cannot be executed on the local database.
    pub unexecutable_steps: Vec<String>,
}

/// Development command for migrations: plan the next migration and return what
/// the engine would do. At this stage, the engine does not create or mutate
/// anything in the database nor in the migrations directory.
///
/// This is where we will return information about potential renamings and other
/// forms of changes that need user decisions in the future, so the CLI can
/// prompt the user.
///
/// This command returns two types of diagnostics: those relating to the
/// migrations history (the dev database is not in sync with the migrations
/// directory), and those relating to destructive operations and operations that
/// need user input, like renamings.
pub struct PlanMigrationCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for PlanMigrationCommand {
    type Input = PlanMigrationInput;

    type Output = PlanMigrationOutput;

    async fn execute<C, D>(_input: &Self::Input, _engine: &MigrationEngine<C, D>) -> super::CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        todo!("planMigration command")
    }
}
