use crate::{json_rpc::types::*, CoreResult};
use migration_connector::{migrations_directory::*, DiffTarget, MigrationConnector};

/// Development command for migrations. Evaluate the data loss induced by the
/// next migration the engine would generate on the main database.
///
/// At this stage, the engine does not create or mutate anything in the database
/// nor in the migrations directory.
pub(crate) async fn evaluate_data_loss(
    input: EvaluateDataLossInput,
    connector: &dyn MigrationConnector,
) -> CoreResult<EvaluateDataLossOutput> {
    let checker = connector.destructive_change_checker();

    error_on_changed_provider(&input.migrations_directory_path, connector.connector_type())?;

    let migrations_from_directory = list_migrations(input.migrations_directory_path.as_ref())?;

    let migration = connector
        .diff(
            DiffTarget::Migrations((&migrations_from_directory).into()),
            DiffTarget::Datamodel((&input.prisma_schema).into()),
        )
        .await?;

    let migration_steps = connector.migration_len(&migration) as u32;
    let diagnostics = checker.check(&migration).await?;

    let warnings = diagnostics
        .warnings
        .into_iter()
        .map(|warning| MigrationFeedback {
            message: warning.description,
            step_index: warning.step_index as u32,
        })
        .collect();

    let unexecutable_steps = diagnostics
        .unexecutable_migrations
        .into_iter()
        .map(|unexecutable| MigrationFeedback {
            message: unexecutable.description,
            step_index: unexecutable.step_index as u32,
        })
        .collect();

    Ok(EvaluateDataLossOutput {
        migration_steps,
        warnings,
        unexecutable_steps,
    })
}
