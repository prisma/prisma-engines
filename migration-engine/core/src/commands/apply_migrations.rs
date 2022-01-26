use crate::{json_rpc::types::*, CoreError, CoreResult};
use migration_connector::{
    migrations_directory::{error_on_changed_provider, list_migrations, MigrationDirectory},
    ConnectorError, MigrationRecord, PersistenceNotInitializedError,
};
use std::{path::Path, time::Instant};
use user_facing_errors::migration_engine::FoundFailedMigrations;

pub(crate) async fn apply_migrations<C>(
    input: &ApplyMigrationsInput,
    connector: &C,
) -> CoreResult<ApplyMigrationsOutput>
where
    C: migration_connector::MigrationConnector,
{
    let start = Instant::now();
    let applier = connector.database_migration_step_applier();
    let migration_persistence = connector.migration_persistence();

    error_on_changed_provider(&input.migrations_directory_path, connector.connector_type())?;

    connector.acquire_lock().await?;

    migration_persistence.initialize().await?;

    let migrations_from_filesystem = list_migrations(Path::new(&input.migrations_directory_path))?;
    let migrations_from_database = migration_persistence
        .list_migrations()
        .await?
        .map_err(PersistenceNotInitializedError::into_connector_error)?;

    detect_failed_migrations(&migrations_from_database)?;

    // We are now on the Happy Path™.
    tracing::debug!("Migration history is OK, applying unapplied migrations.");
    let unapplied_migrations: Vec<&MigrationDirectory> = migrations_from_filesystem
        .iter()
        .filter(|fs_migration| {
            !migrations_from_database
                .iter()
                .filter(|db_migration| db_migration.rolled_back_at.is_none())
                .any(|db_migration| fs_migration.migration_name() == db_migration.migration_name)
        })
        .collect();

    let analysis_duration_ms = Instant::now().duration_since(start).as_millis() as u64;
    tracing::info!(analysis_duration_ms, "Analysis run in {}ms", analysis_duration_ms,);

    let mut applied_migration_names: Vec<String> = Vec::with_capacity(unapplied_migrations.len());
    let apply_migrations_start = Instant::now();

    for unapplied_migration in unapplied_migrations {
        let span = tracing::info_span!(
            "Applying migration",
            migration_name = unapplied_migration.migration_name(),
        );
        let _span = span.enter();
        let migration_apply_start = Instant::now();

        let script = unapplied_migration
            .read_migration_script()
            .map_err(ConnectorError::from)?;

        tracing::info!(
            script = script.as_str(),
            "Applying `{}`",
            unapplied_migration.migration_name()
        );

        let migration_id = migration_persistence
            .record_migration_started(unapplied_migration.migration_name(), &script)
            .await?;

        match applier
            .apply_script(unapplied_migration.migration_name(), &script)
            .await
        {
            Ok(()) => {
                tracing::debug!("Successfully applied the script.");
                migration_persistence.record_successful_step(&migration_id).await?;
                migration_persistence.record_migration_finished(&migration_id).await?;
                applied_migration_names.push(unapplied_migration.migration_name().to_owned());
                let migration_duration_ms = Instant::now().duration_since(migration_apply_start).as_millis() as u64;
                tracing::info!(
                    migration_duration_ms = migration_duration_ms,
                    "Migration executed in {}ms",
                    migration_duration_ms
                );
            }
            Err(err) => {
                tracing::debug!("Failed to apply the script.");

                let logs = err.to_string();

                migration_persistence.record_failed_step(&migration_id, &logs).await?;

                return Err(err);
            }
        }
    }

    let apply_migrations_ms = Instant::now().duration_since(apply_migrations_start).as_millis() as u64;
    tracing::info!(
        apply_migrations_duration_ms = apply_migrations_ms,
        "All the migrations executed in {}ms",
        apply_migrations_ms
    );

    Ok(ApplyMigrationsOutput {
        applied_migration_names,
    })
}

fn detect_failed_migrations(migrations_from_database: &[MigrationRecord]) -> CoreResult<()> {
    use std::fmt::Write as _;

    tracing::debug!("Checking for failed migrations.");

    let mut failed_migrations = migrations_from_database
        .iter()
        .filter(|migration| migration.finished_at.is_none() && migration.rolled_back_at.is_none())
        .peekable();

    if failed_migrations.peek().is_none() {
        return Ok(());
    }

    let mut details = String::new();

    for failed_migration in failed_migrations {
        writeln!(
            details,
            "The `{name}` migration started at {started_at} failed with the following logs:\n{logs}",
            name = failed_migration.migration_name,
            started_at = failed_migration.started_at,
            logs = if let Some(logs) = &failed_migration.logs {
                format!("with the following logs:\n{}", logs)
            } else {
                String::new()
            }
        )
        .unwrap();
    }

    Err(CoreError::user_facing(FoundFailedMigrations { details }))
}
