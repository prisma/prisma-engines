use super::{CommandResult, MigrationCommand};
use crate::migration_engine::MigrationEngine;
use serde::{Deserialize, Serialize};

/// The input to the `planMigration` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlanMigrationInput {}

/// The output of the `planMigration` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlanMigrationOutput {}

/// Development command for migrations. At this stage, the engine does not
/// create or mutate anything in the database nor in the migrations directory.
///
/// This is where we will return information about potential renamings and other
/// forms of changes that need user decisions in the future, so the CLI can
/// prompt the user.
pub struct PlanMigrationCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for PlanMigrationCommand {
    type Input = PlanMigrationInput;

    type Output = PlanMigrationOutput;

    async fn execute<C, D>(_input: &Self::Input, _engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        unreachable!("PlanMigration command")
    }
}
