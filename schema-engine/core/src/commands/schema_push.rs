use crate::{json_rpc::types::*, parse_schema_multi, CoreResult, SchemaContainerExt};
use schema_connector::{ConnectorError, DiffTarget, SchemaConnector};
use tracing_futures::Instrument;

/// Command to bring the local database in sync with the prisma schema, without
/// interacting with the migrations directory nor the migrations table.
pub async fn schema_push(input: SchemaPushInput, connector: &mut dyn SchemaConnector) -> CoreResult<SchemaPushOutput> {
    let sources = input.schema.to_psl_input();
    let datamodel = parse_schema_multi(&sources)?;

    if let Some(err) = connector.check_database_version_compatibility(&datamodel) {
        return Err(ConnectorError::user_facing(err));
    };

    let to = connector
        .database_schema_from_diff_target(DiffTarget::Datamodel(sources), None, None)
        .instrument(tracing::info_span!("Calculate `to`"))
        .await?;

    let namespaces = connector.extract_namespaces(&to);

    // TODO(MultiSchema): we may need to do something similar to
    // namespaces_and_preview_features_from_diff_targets here as well,
    // particulalry if it's not correctly setting the preview features flags.
    let from = connector
        .database_schema_from_diff_target(DiffTarget::Database, None, namespaces)
        .instrument(tracing::info_span!("Calculate `from`"))
        .await?;
    let database_migration = connector.diff(from, to);

    tracing::debug!(migration = connector.migration_summary(&database_migration).as_str());

    let checks = connector
        .destructive_change_checker()
        .check(&database_migration)
        .await?;

    let executed_steps = match (checks.unexecutable_migrations.len(), checks.warnings.len(), input.force) {
        (unexecutable, _, _) if unexecutable > 0 => {
            tracing::warn!(unexecutable = ?checks.unexecutable_migrations, "Aborting migration because at least one unexecutable step was detected.");

            0
        }
        (0, 0, _) | (0, _, true) => connector.apply_migration(&database_migration).await?,
        _ => {
            tracing::info!(
                "The migration was not applied because it triggered warnings and the force flag was not passed."
            );

            0
        }
    };

    let warnings = checks.warnings.into_iter().map(|warning| warning.description).collect();

    let unexecutable = checks
        .unexecutable_migrations
        .into_iter()
        .map(|unexecutable| unexecutable.description)
        .collect();

    Ok(SchemaPushOutput {
        executed_steps,
        warnings,
        unexecutable,
    })
}
