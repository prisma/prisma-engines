use std::sync::Arc;

use crate::{json_rpc::types::*, CoreResult};
use datamodel::schema_ast::source_file::SourceFile;
use migration_connector::{migrations_directory::*, DiffTarget, MigrationConnector};

/// Development command for migrations. Evaluate the data loss induced by the
/// next migration the engine would generate on the main database.
///
/// At this stage, the engine does not create or mutate anything in the database
/// nor in the migrations directory.
pub async fn evaluate_data_loss(
    input: EvaluateDataLossInput,
    connector: &mut dyn MigrationConnector,
) -> CoreResult<EvaluateDataLossOutput> {
    error_on_changed_provider(&input.migrations_directory_path, connector.connector_type())?;
    let source_file = SourceFile::new_allocated(Arc::from(input.prisma_schema.into_boxed_str()));

    let migrations_from_directory = list_migrations(input.migrations_directory_path.as_ref())?;

    let from = connector
        .database_schema_from_diff_target(DiffTarget::Migrations(&migrations_from_directory), None)
        .await?;
    let to = connector
        .database_schema_from_diff_target(DiffTarget::Datamodel(source_file), None)
        .await?;
    let migration = connector.diff(from, to)?;

    let migration_steps = connector.migration_len(&migration) as u32;
    let diagnostics = connector.destructive_change_checker().check(&migration).await?;

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
