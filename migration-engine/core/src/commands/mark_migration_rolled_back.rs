use super::MigrationCommand;
use crate::{migration_engine::MigrationEngine, CoreError, CoreResult};
use serde::Deserialize;
use std::collections::HashMap;

/// The input to the `markMigrationRolledBack` command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkMigrationRolledBackInput {
    /// The name of the migration to mark rolled back.
    pub migration_name: String,
}

/// The output of the `markMigrationRolledBack` command.
pub type MarkMigrationRolledBackOutput = HashMap<(), ()>;

/// Mark a migration as rolled back.
pub struct MarkMigrationRolledBackCommand;

#[async_trait::async_trait]
impl MigrationCommand for MarkMigrationRolledBackCommand {
    type Input = MarkMigrationRolledBackInput;
    type Output = MarkMigrationRolledBackOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        // We should take a lock on the migrations table.

        let persistence = engine.connector().new_migration_persistence();

        let all_migrations = persistence.list_migrations().await?.map_err(|_err| {
            CoreError::Generic(anyhow::anyhow!(
                "Invariant violation: called markMigrationRolledBack on a database without migrations table."
            ))
        })?;

        let relevant_migrations: Vec<_> = all_migrations
            .into_iter()
            .filter(|migration| migration.migration_name == input.migration_name)
            .collect();

        if relevant_migrations.is_empty() {
            return Err(CoreError::Generic(anyhow::anyhow!(
                "Migration `{}` cannot be rolled back because it was never applied to the database.",
                &input.migration_name
            )));
        }

        if relevant_migrations
            .iter()
            .all(|migration| migration.finished_at.is_some())
        {
            return Err(CoreError::Generic(anyhow::anyhow!(
                "Migration `{}` cannot be rolled back because it is not in a failed state.",
                &input.migration_name
            )));
        }

        let migrations_to_roll_back = relevant_migrations
            .iter()
            .filter(|migration| migration.finished_at.is_none() && migration.rolled_back_at.is_none());

        for migration in migrations_to_roll_back {
            tracing::info!(
                migration_id = migration.id.as_str(),
                migration_name = migration.migration_name.as_str(),
                "Marking migration as rolled back."
            );
            persistence.mark_migration_rolled_back_by_id(&migration.id).await?;
        }

        Ok(Default::default())
    }
}
