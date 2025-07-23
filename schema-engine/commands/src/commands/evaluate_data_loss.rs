use crate::{CoreResult, MigrationSchemaCache, SchemaContainerExt, json_rpc::types::*};
use schema_connector::{SchemaConnector, migrations_directory::*};

/// Development command for migrations. Evaluate the data loss induced by the
/// next migration the engine would generate on the main database.
///
/// At this stage, the engine does not create or mutate anything in the database
/// nor in the migrations directory.
pub async fn evaluate_data_loss(
    input: EvaluateDataLossInput,
    connector: &mut dyn SchemaConnector,
    migration_schema_cache: &mut MigrationSchemaCache,
) -> CoreResult<EvaluateDataLossOutput> {
    error_on_changed_provider(&input.migrations_list.lockfile, connector.connector_type())?;
    let sources: Vec<_> = input.schema.to_psl_input();

    let migrations = Migrations::from_migration_list(&input.migrations_list);
    let dialect = connector.schema_dialect();
    let filter: schema_connector::SchemaFilter = input.filters.into();

    let to = dialect.schema_from_datamodel(sources, connector.default_namespace())?;

    let from = migration_schema_cache
        .get_or_insert(&input.migrations_list.migration_directories, || async {
            // We only consider the namespaces present in the "to" schema aka the PSL file for the introspection of the "from" schema.
            // So when the user removes a previously existing namespace from their PSL file we will not introspect that namespace in the database.
            let namespaces = dialect.extract_namespaces(&to);
            connector.schema_from_migrations(&migrations, namespaces, &filter).await
        })
        .await?;

    let migration = dialect.diff(from, to, &filter);

    let migration_steps = dialect.migration_len(&migration) as u32;
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
