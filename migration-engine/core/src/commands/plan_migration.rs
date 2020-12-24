use super::MigrationCommand;
use crate::{api::MigrationApi, CoreResult};
use migration_connector::MigrationConnector;
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

    async fn execute<C: MigrationConnector>(
        _input: &Self::Input,
        _engine: &MigrationApi<C>,
    ) -> CoreResult<Self::Output> {
        unreachable!("PlanMigration command")
    }
}
