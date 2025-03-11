use crate::{json_rpc::types::*, CoreError, CoreResult, SchemaContainerExt};
use crosstarget_utils::time::format_utc_now;
use schema_connector::{migrations_directory::*, DiffTarget, SchemaConnector};
use user_facing_errors::schema_engine::MigrationNameTooLong;

/// Create a directory name for a new migration.
pub fn generate_migration_directory_name(migration_name: &str) -> String {
    let timestamp = format_utc_now("%Y%m%d%H%M%S");
    let directory_name = format!("{}_{}", timestamp, migration_name);
    directory_name
}

/// Create a new migration.
pub async fn create_migration(
    input: CreateMigrationInput,
    connector: &mut dyn SchemaConnector,
) -> CoreResult<CreateMigrationOutput> {
    let connector_type = connector.connector_type();

    if input.migration_name.len() > 200 {
        return Err(CoreError::user_facing(MigrationNameTooLong));
    }

    // Check for provider switch
    error_on_changed_provider(&input.migrations_list.lockfile, connector_type)?;

    let generated_migration_name = generate_migration_directory_name(&input.migration_name);

    // Infer the migration.
    let previous_migrations = list_migrations(input.migrations_list.migration_directories);
    let sources: Vec<_> = input.schema.to_psl_input();
    // We need to start with the 'to', which is the Schema, in order to grab the
    // namespaces, in case we've got MultiSchema enabled.
    let to = connector
        .database_schema_from_diff_target(DiffTarget::Datamodel(sources), None, None)
        .await?;

    let namespaces = connector.extract_namespaces(&to);
    // We pass the namespaces here, because we want to describe all of these namespaces.
    let from = connector
        .database_schema_from_diff_target(DiffTarget::Migrations(&previous_migrations), None, namespaces)
        .await?;
    let migration = connector.diff(from, to);

    let extension = connector.migration_file_extension().to_owned();

    if connector.migration_is_empty(&migration) && !input.draft {
        tracing::info!("Database is up-to-date, returning without creating new migration.");

        return Ok(CreateMigrationOutput {
            connector_type: connector_type.to_owned(),
            generated_migration_name,
            migration_script: None,
            extension,
        });
    }

    let destructive_change_diagnostics = connector.destructive_change_checker().pure_check(&migration);

    let migration_script = connector.render_script(&migration, &destructive_change_diagnostics)?;

    Ok(CreateMigrationOutput {
        connector_type: connector_type.to_owned(),
        generated_migration_name,
        migration_script: Some(migration_script),
        extension,
    })
}
