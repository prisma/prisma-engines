use crate::{CoreError, CoreResult};
use migration_connector::{
    migrations_directory::{error_on_changed_provider, MigrationDirectory},
    MigrationConnector,
};
use serde::Deserialize;
use std::{collections::HashMap, path::Path};
use user_facing_errors::migration_engine::{MigrationAlreadyApplied, MigrationToMarkAppliedNotFound};

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
pub async fn mark_migration_applied(
    input: &MarkMigrationAppliedInput,
    connector: &dyn MigrationConnector,
) -> CoreResult<MarkMigrationAppliedOutput> {
    let persistence = connector.migration_persistence();

    error_on_changed_provider(&input.migrations_directory_path, connector.connector_type())?;

    connector.acquire_lock().await?;

    let migration_directory =
        MigrationDirectory::new(Path::new(&input.migrations_directory_path).join(&input.migration_name));

    let script = migration_directory.read_migration_script().map_err(|_err| {
        CoreError::user_facing(MigrationToMarkAppliedNotFound {
            migration_name: input.migration_name.clone(),
        })
    })?;

    let relevant_migrations = match persistence.list_migrations().await? {
        Ok(migrations) => migrations
            .into_iter()
            .filter(|migration| migration.migration_name == input.migration_name)
            .collect(),
        Err(_) => {
            persistence.baseline_initialize().await?;

            vec![]
        }
    };

    if relevant_migrations
        .iter()
        .any(|migration| migration.finished_at.is_some())
    {
        return Err(CoreError::user_facing(MigrationAlreadyApplied {
            migration_name: input.migration_name.clone(),
        }));
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
