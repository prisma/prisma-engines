use crate::{parse_schema, CoreResult};
use migration_connector::{migrations_directory::*, DiffTarget, MigrationConnector};
use serde::{Deserialize, Serialize};

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
    /// The number of migration steps the engine would generate.
    pub migration_steps: usize,
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

/// Development command for migrations. Evaluate the data loss induced by the
/// next migration the engine would generate on the main database.
///
/// At this stage, the engine does not create or mutate anything in the database
/// nor in the migrations directory.
pub(crate) async fn evaluate_data_loss(
    input: &EvaluateDataLossInput,
    connector: &dyn MigrationConnector,
) -> CoreResult<EvaluateDataLossOutput> {
    let checker = connector.destructive_change_checker();

    error_on_changed_provider(&input.migrations_directory_path, connector.connector_type())?;

    let migrations_from_directory = list_migrations(input.migrations_directory_path.as_ref())?;
    let target_schema = parse_schema(&input.prisma_schema)?;

    let migration = connector
        .diff(
            DiffTarget::Migrations(&migrations_from_directory),
            DiffTarget::Datamodel((&target_schema.0, &target_schema.1)),
        )
        .await?;

    let migration_steps = connector.migration_len(&migration);
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
        migration_steps,
        warnings,
        unexecutable_steps,
    })
}
