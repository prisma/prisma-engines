use crate::{CoreError, CoreResult, MigrationSchemaCache, SchemaContainerExt, json_rpc::types::*};
use crosstarget_utils::time::format_utc_now;
use schema_connector::{SchemaConnector, migrations_directory::*};
use user_facing_errors::schema_engine::MigrationNameTooLong;

/// Create a directory name for a new migration.
pub fn generate_migration_directory_name(migration_name: &str) -> String {
    let timestamp = format_utc_now("%Y%m%d%H%M%S");
    let directory_name = format!("{timestamp}_{migration_name}");
    directory_name
}

/// Create a new migration.
pub async fn create_migration(
    input: CreateMigrationInput,
    connector: &mut dyn SchemaConnector,
    migration_schema_cache: &mut MigrationSchemaCache,
) -> CoreResult<CreateMigrationOutput> {
    let connector_type = connector.connector_type();

    if input.migration_name.len() > 200 {
        return Err(CoreError::user_facing(MigrationNameTooLong));
    }

    // Check for provider switch
    error_on_changed_provider(&input.migrations_list.lockfile, connector_type)?;

    let generated_migration_name = generate_migration_directory_name(&input.migration_name);

    // Infer the migration.
    let previous_migrations = list_migrations(input.migrations_list.migration_directories.clone());
    let sources: Vec<_> = input.schema.to_psl_input();
    let dialect = connector.schema_dialect();
    // We need to start with the 'to', which is the Schema, in order to grab the
    // namespaces, in case we've got MultiSchema enabled.
    let to = dialect.schema_from_datamodel(sources, input.filters.into())?;

    let from = migration_schema_cache
        .get_or_insert(&input.migrations_list.migration_directories, || async {
            let namespaces = dialect.extract_namespaces(&to);
            // We pass the namespaces here, because we want to describe all of these namespaces.
            connector.schema_from_migrations(&previous_migrations, namespaces).await
        })
        .await?;

    let migration = dialect.diff(from, to);

    let extension = dialect.migration_file_extension().to_owned();

    if dialect.migration_is_empty(&migration) && !input.draft {
        tracing::info!("Database is up-to-date, returning without creating new migration.");

        return Ok(CreateMigrationOutput {
            connector_type: connector_type.to_owned(),
            generated_migration_name,
            migration_script: None,
            extension,
        });
    }

    let destructive_change_diagnostics = connector.destructive_change_checker().pure_check(&migration);

    let migration_script = dialect.render_script(&migration, &destructive_change_diagnostics)?;

    Ok(CreateMigrationOutput {
        connector_type: connector_type.to_owned(),
        generated_migration_name,
        migration_script: Some(migration_script),
        extension,
    })
}
