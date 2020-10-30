use crate::migration_engine::MigrationEngine;
use crate::{commands::command::*, CoreResult};
use migration_connector::steps::*;
use migration_connector::*;
use serde::Serialize;

pub struct ListMigrationsCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for ListMigrationsCommand {
    type Input = serde_json::Value;
    type Output = Vec<ListMigrationsOutput>;

    async fn execute<C, D>(_input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let migration_persistence = engine.connector().migration_persistence();

        let migrations: Self::Output = migration_persistence
            .load_all()
            .await?
            .into_iter()
            .map(convert_migration_to_list_migration_steps_output)
            .collect();

        tracing::info!(
            "Returning {migrations_count} migrations ({pending_count} pending).",
            migrations_count = migrations.len(),
            pending_count = migrations.iter().filter(|mig| mig.status.is_pending()).count(),
        );

        Ok(migrations)
    }
}

pub fn convert_migration_to_list_migration_steps_output(migration: Migration) -> ListMigrationsOutput {
    ListMigrationsOutput {
        id: migration.name,
        datamodel_steps: migration.datamodel_steps,
        database_steps: vec![],
        status: migration.status,
        datamodel: migration.datamodel_string,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMigrationsOutput {
    pub id: String,
    pub datamodel_steps: Vec<MigrationStep>,
    pub database_steps: Vec<PrettyDatabaseMigrationStep>,
    pub status: MigrationStatus,
    pub datamodel: String,
}
