use super::MigrationCommand;
use crate::{migration_engine::MigrationEngine, parse_datamodel, CoreResult};
use migration_connector::list_migrations;
use serde::{Deserialize, Serialize};

/// Development command for migrations. Evaluate the data loss induced by the
/// next migration the engine would generate on the main database.
///
/// At this stage, the engine does not create or mutate anything in the database
/// nor in the migrations directory.
pub struct EvaluateDataLoss;

/// The input to the `evaluateDataLoss` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateDataLossInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
    /// The prisma schema to migrate to.
    pub prisma_schema: String,
}

/// The output of the `evaluateDataLoss` command.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateDataLossOutput {
    /// The migration steps the engine would generate.
    pub migration_steps: Vec<String>,
    /// Destructive change warnings for the local database. These are the
    /// warnings *for the migration that would be generated*. This does not
    /// include other potentially yet unapplied migrations.
    pub warnings: Vec<MigrationFeedback>,
    /// Steps that cannot be executed on the local database in the migration
    /// that would be generated.
    pub unexecutable_steps: Vec<MigrationFeedback>,
}

/// A data loss warning or an unexecutable migration error, associated with the step that triggered it.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationFeedback {
    /// The human-readable message.
    pub message: String,
    /// The index of the step this pertains to.
    pub step_index: usize,
}

#[async_trait::async_trait]
impl MigrationCommand for EvaluateDataLoss {
    type Input = EvaluateDataLossInput;
    type Output = EvaluateDataLossOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
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

        Ok(EvaluateDataLossOutput {
            migration_steps: rendered_migration_steps,
            warnings,
            unexecutable_steps,
        })
    }
}
