use super::{CommandResult, MigrationCommand};
use crate::{migration_engine::MigrationEngine, parse_datamodel};
use migration_connector::list_migrations;
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
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanMigrationOutput {
    /// The migration steps we would generate.
    pub migration_steps: Vec<String>,
    /// Destructive change warnings for the local database. These are the
    /// warnings *for the migration that would be generated*. This does not
    /// include other potentially yet unapplied migrations.
    pub warnings: Vec<MigrationFeedback>,
    /// Steps that cannot be executed on the local database in the migration
    /// that would be generated.
    pub unexecutable_steps: Vec<MigrationFeedback>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationFeedback {
    /// The human-readable message.
    pub message: String,
    /// The index of the step this pertains to.
    pub step_index: usize,
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
///
/// _Implementation note_: in this command, the engine assumes the development
/// database is up-to-date with the existing migration history from the
/// migrations directory. This is enforced by the CLI using other commands,
/// mainly diagnoseMigrationHistory. If it is not in sync with the migrations
/// history, the warnings and errors we return here will be off. There will be
/// no warnings generated for already present but unapplied migrations, which
/// may contain wrong or destructive changes.
pub struct PlanMigrationCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for PlanMigrationCommand {
    type Input = PlanMigrationInput;

    type Output = PlanMigrationOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let inferrer = connector.database_migration_inferrer();
        let applier = connector.database_migration_step_applier();
        let checker = connector.destructive_change_checker();

        let migrations_from_directory = list_migrations(input.migrations_directory_path.as_ref())?;
        let target_schema = parse_datamodel(&input.prisma_schema)?;

        let migration = inferrer
            .infer_next_migration(&migrations_from_directory, &target_schema)
            .await?;

        let rendered_migration_steps = applier
            .render_steps_pretty(&migration)?
            .into_iter()
            .map(|pretty_step| pretty_step.raw)
            .collect();

        let diagnostics = checker.check(&migration).await?;

        let warnings = diagnostics
            .warnings
            .into_iter()
            .map(|warning| MigrationFeedback {
                message: warning.description,
                step_index: warning.step_index,
            })
            .collect();

        let unexecutable_steps = diagnostics
            .unexecutable_migrations
            .into_iter()
            .map(|unexecutable| MigrationFeedback {
                message: unexecutable.description,
                step_index: unexecutable.step_index,
            })
            .collect();

        Ok(PlanMigrationOutput {
            migration_steps: rendered_migration_steps,
            warnings,
            unexecutable_steps,
        })
    }
}
