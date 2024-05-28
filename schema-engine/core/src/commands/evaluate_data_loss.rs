use crate::{json_rpc::types::*, CoreResult, SchemaContainerExt};
use schema_connector::{migrations_directory::*, DiffTarget, SchemaConnector};

/// Development command for migrations. Evaluate the data loss induced by the
/// next migration the engine would generate on the main database.
///
/// At this stage, the engine does not create or mutate anything in the database
/// nor in the migrations directory.
pub async fn evaluate_data_loss(
    input: EvaluateDataLossInput,
    connector: &mut dyn SchemaConnector,
) -> CoreResult<EvaluateDataLossOutput> {
    error_on_changed_provider(&input.migrations_directory_path, connector.connector_type())?;
    let sources: Vec<_> = input.schema.to_psl_input();

    let migrations_from_directory = list_migrations(input.migrations_directory_path.as_ref())?;

    let to = connector
        .database_schema_from_diff_target(DiffTarget::Datamodel(sources), None, None)
        .await?;

    let namespaces = connector.extract_namespaces(&to);

    // TODO(MultiSchema): we may need to do something similar to
    // namespaces_and_preview_features_from_diff_targets here as well,
    // particulalry if it's not correctly setting the preview features flags.
    let from = connector
        .database_schema_from_diff_target(DiffTarget::Migrations(&migrations_from_directory), None, namespaces)
        .await?;
    let migration = connector.diff(from, to);

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
