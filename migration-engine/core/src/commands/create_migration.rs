use crate::{json_rpc::types::*, CoreError, CoreResult};
use migration_connector::{migrations_directory::*, DiffTarget, MigrationConnector};
use std::path::Path;
use user_facing_errors::migration_engine::MigrationNameTooLong;

/// Create a new migration.
pub async fn create_migration(
    input: CreateMigrationInput,
    connector: &mut dyn MigrationConnector,
) -> CoreResult<CreateMigrationOutput> {
    let connector_type = connector.connector_type();

    if input.migration_name.len() > 200 {
        return Err(CoreError::user_facing(MigrationNameTooLong));
    }

    // Check for provider switch
    error_on_changed_provider(&input.migrations_directory_path, connector_type)?;

    // Infer the migration.
    let previous_migrations = list_migrations(Path::new(&input.migrations_directory_path))?;

    let from = connector
        .database_schema_from_diff_target(DiffTarget::Migrations(&previous_migrations), None)
        .await?;
    let to = connector
        .database_schema_from_diff_target(DiffTarget::Datamodel(&input.prisma_schema), None)
        .await?;
    let migration = connector.diff(from, to)?;

    if connector.migration_is_empty(&migration) && !input.draft {
        tracing::info!("Database is up-to-date, returning without creating new migration.");

        return Ok(CreateMigrationOutput {
            generated_migration_name: None,
        });
    }

    let destructive_change_diagnostics = connector.destructive_change_checker().pure_check(&migration);

    let migration_script = connector.render_script(&migration, &destructive_change_diagnostics)?;

    // Write the migration script to a file.
    let directory = create_migration_directory(Path::new(&input.migrations_directory_path), &input.migration_name)
        .map_err(|_| CoreError::from_msg("Failed to create a new migration directory.".into()))?;

    directory
        .write_migration_script(&migration_script, connector.migration_file_extension())
        .map_err(|err| {
            CoreError::from_msg(format!(
                "Failed to write the migration script to `{:?}`\n{}",
                directory.path(),
                err,
            ))
        })?;

    write_migration_lock_file(&input.migrations_directory_path, connector_type).map_err(|err| {
        CoreError::from_msg(format!(
            "Failed to write the migration lock file to `{:?}`\n{}",
            &input.migrations_directory_path, err
        ))
    })?;

    Ok(CreateMigrationOutput {
        generated_migration_name: Some(directory.migration_name().to_owned()),
    })
}
