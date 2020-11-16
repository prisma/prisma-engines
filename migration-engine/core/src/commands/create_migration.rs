use super::MigrationCommand;
use crate::{migration_engine::MigrationEngine, parse_datamodel, CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let database_migration_inferrer = engine.connector().database_migration_inferrer();
        let applier = engine.connector().database_migration_step_applier();
        let checker = engine.connector().destructive_change_checker();

        // Infer the migration.
        let previous_migrations = migration_connector::list_migrations(&Path::new(&input.migrations_directory_path))?;
        let target_schema = parse_datamodel(&input.prisma_schema)?;

        let migration = database_migration_inferrer
            .infer_next_migration(&previous_migrations, &target_schema)
            .await?;

        if migration.is_empty() && !input.draft {
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
        .map_err(|_| CoreError::Generic(anyhow::anyhow!("Failed to create a new migration directory.")))?;
        directory
            .write_migration_script(&migration_script, D::FILE_EXTENSION)
            .map_err(|err| {
                CoreError::Generic(anyhow::Error::new(err).context(format!(
                    "Failed to write the migration script to `{:?}`",
                    directory.path(),
                )))
            })?;

        Ok(CreateMigrationOutput {
            generated_migration_name: Some(directory.migration_name().to_owned()),
        })
    }
}
