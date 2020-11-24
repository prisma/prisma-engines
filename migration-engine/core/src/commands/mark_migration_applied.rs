use super::MigrationCommand;
use crate::{migration_engine::MigrationEngine, CoreError, CoreResult};
use migration_connector::MigrationDirectory;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};
use user_facing_errors::migration_engine::MigrationAlreadyApplied;

/// The input to the `markMigrationApplied` command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkMigrationAppliedInput {
    /// The name of the migration to mark applied.
    pub migration_name: String,
    /// The path to the root of the migrations directory.
    pub migrations_directory_path: String,
}

/// The output of the `markMigrationApplied` command.
pub type MarkMigrationAppliedOutput = HashMap<(), ()>;

/// Mark a migration as applied.
pub struct MarkMigrationAppliedCommand;

#[async_trait::async_trait]
impl MigrationCommand for MarkMigrationAppliedCommand {
    type Input = MarkMigrationAppliedInput;

    type Output = MarkMigrationAppliedOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        // We should take a lock on the migrations table.

        let persistence = engine.connector().new_migration_persistence();

        let migration_directory =
            MigrationDirectory::new(Path::new(&input.migrations_directory_path).join(&input.migration_name));
        let script = migration_directory
            .read_migration_script()
            .map_err(|err| CoreError::Generic(err.into()))?;

        let relevant_migrations = match persistence.list_migrations().await? {
            Ok(migrations) => migrations
                .into_iter()
                .filter(|migration| migration.migration_name == input.migration_name)
                .collect(),
            Err(_) => {
                persistence.initialize(true).await?;

                vec![]
            }
        };

        if relevant_migrations
            .iter()
            .any(|migration| migration.finished_at.is_some())
        {
            return Err(CoreError::UserFacing(user_facing_errors::KnownError::new(
                MigrationAlreadyApplied {
                    migration_name: input.migration_name.clone(),
                },
            )));
        }

        let migrations_to_mark_rolled_back = relevant_migrations
            .iter()
            .filter(|migration| migration.finished_at.is_none() && migration.rolled_back_at.is_none());

        for migration in migrations_to_mark_rolled_back {
            persistence.mark_migration_rolled_back_by_id(&migration.id).await?;
        }

        persistence
            .mark_migration_applied(migration_directory.migration_name(), &script)
            .await?;

        Ok(Default::default())
    }
}
