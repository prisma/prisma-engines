use crate::{CoreError, CoreResult, json_rpc::types::*};
use schema_connector::{
    SchemaConnector,
    migrations_directory::{MigrationDirectory, error_on_changed_provider},
};
use user_facing_errors::schema_engine::{MigrationAlreadyApplied, MigrationToMarkAppliedNotFound};

/// Mark a migration as applied.
pub async fn mark_migration_applied(
    input: MarkMigrationAppliedInput,
    connector: &mut dyn SchemaConnector,
) -> CoreResult<MarkMigrationAppliedOutput> {
    error_on_changed_provider(&input.migrations_list.lockfile, connector.connector_type())?;

    connector.acquire_lock().await?;

    let migration_directory = input
        .migrations_list
        .migration_directories
        .into_iter()
        .map(MigrationDirectory::new)
        .find(|dir| input.migration_name == dir.migration_name())
        .ok_or_else(|| {
            CoreError::user_facing(MigrationToMarkAppliedNotFound {
                migration_name: input.migration_name.clone(),
            })
        })?;

    let script = migration_directory.read_migration_script().map_err(|_err| {
        CoreError::user_facing(MigrationToMarkAppliedNotFound {
            migration_name: input.migration_name.clone(),
        })
    })?;

    let relevant_migrations = match connector.migration_persistence().list_migrations().await? {
        Ok(migrations) => migrations
            .into_iter()
            .filter(|migration| migration.migration_name == input.migration_name)
            .collect(),
        Err(_) => {
            connector.migration_persistence().baseline_initialize().await?;

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
        connector
            .migration_persistence()
            .mark_migration_rolled_back_by_id(&migration.id)
            .await?;
    }

    connector
        .migration_persistence()
        .mark_migration_applied(migration_directory.migration_name(), &script)
        .await?;

    Ok(MarkMigrationAppliedOutput {})
}
