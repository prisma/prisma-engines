use super::MigrationCommand;
use crate::{parse_schema, CoreError, CoreResult};
use migration_connector::{DatabaseMigrationMarker, MigrationConnector};
use serde::{Deserialize, Serialize};
use std::path::Path;
use user_facing_errors::migration_engine::MigrationNameTooLong;

/// Create and potentially apply a new migration.
pub struct CreateMigrationCommand;

/// The input to the `createMigration` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateMigrationInput {
    /// The filesystem path of the migrations directory to use.
    pub migrations_directory_path: String,
    /// The current prisma schema to use as a target for the generated migration.
    pub prisma_schema: String,
    /// The user-given name for the migration. This will be used in the migration directory.
    pub migration_name: String,
    /// If true, always generate a migration, but do not apply.
    pub draft: bool,
}

/// The output of the `createMigration` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateMigrationOutput {
    /// The name of the newly generated migration directory, if any.
    pub generated_migration_name: Option<String>,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for CreateMigrationCommand {
    type Input = CreateMigrationInput;

    type Output = CreateMigrationOutput;

    async fn execute<C: MigrationConnector>(input: &Self::Input, connector: &C) -> CoreResult<Self::Output> {
        let database_migration_inferrer = connector.database_migration_inferrer();
        let applier = connector.database_migration_step_applier();
        let checker = connector.destructive_change_checker();
        let connector_type = connector.connector_type();

        if input.migration_name.len() > 200 {
            return Err(CoreError::user_facing(MigrationNameTooLong));
        }

        // Check for provider switch
        migration_connector::error_on_changed_provider(&input.migrations_directory_path, connector_type)?;

        // Infer the migration.
        let previous_migrations = migration_connector::list_migrations(&Path::new(&input.migrations_directory_path))?;
        let target_schema = parse_schema(&input.prisma_schema)?;

        let migration = database_migration_inferrer
            .infer_next_migration(&previous_migrations, (&target_schema.0, &target_schema.1))
            .await?;

        if migration.is_empty() && !input.draft {
            tracing::info!("Database is up-to-date, returning without creating new migration.");

            return Ok(CreateMigrationOutput {
                generated_migration_name: None,
            });
        }

        let destructive_change_diagnostics = checker.pure_check(&migration);

        let migration_script = applier.render_script(&migration, &destructive_change_diagnostics);

        // Write the migration script to a file.
        let directory = migration_connector::create_migration_directory(
            &Path::new(&input.migrations_directory_path),
            &input.migration_name,
        )
        .map_err(|_| CoreError::from_msg("Failed to create a new migration directory.".into()))?;

        directory
            .write_migration_script(&migration_script, C::DatabaseMigration::FILE_EXTENSION)
            .map_err(|err| {
                CoreError::from_msg(format!(
                    "Failed to write the migration script to `{:?}`\n{}",
                    directory.path(),
                    err,
                ))
            })?;

        migration_connector::write_migration_lock_file(&input.migrations_directory_path, connector_type).map_err(
            |err| {
                CoreError::from_msg(format!(
                    "Failed to write the migration lock file to `{:?}`\n{}",
                    &input.migrations_directory_path, err
                ))
            },
        )?;

        Ok(CreateMigrationOutput {
            generated_migration_name: Some(directory.migration_name().to_owned()),
        })
    }
}
