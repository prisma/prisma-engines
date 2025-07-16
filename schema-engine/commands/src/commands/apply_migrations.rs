use crate::{CoreError, CoreResult, json_rpc::types::*};
use crosstarget_utils::time::ElapsedTimeCounter;
use schema_connector::{
    ConnectorError, MigrationRecord, Namespaces, PersistenceNotInitializedError, SchemaConnector,
    migrations_directory::{MigrationDirectory, error_on_changed_provider, list_migrations},
};
use tracing::Instrument;
use user_facing_errors::schema_engine::FoundFailedMigrations;

pub async fn apply_migrations(
    input: ApplyMigrationsInput,
    connector: &mut dyn SchemaConnector,
    namespaces: Option<Namespaces>,
) -> CoreResult<ApplyMigrationsOutput> {
    let start = ElapsedTimeCounter::start();

    error_on_changed_provider(&input.migrations_list.lockfile, connector.connector_type())?;
    let migrations_from_filesystem = list_migrations(input.migrations_list.migration_directories);

    connector.acquire_lock().await?;
    connector
        .migration_persistence()
        .initialize(namespaces, input.filters.into())
        .await?;

    let migrations_from_database = connector
        .migration_persistence()
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
                .any(|db_migration| db_migration.migration_name == fs_migration.migration_name())
        })
        .collect();

    let analysis_duration_ms = start.elapsed_time().as_millis() as u64;
    tracing::info!(analysis_duration_ms, "Analysis run in {}ms", analysis_duration_ms,);

    let mut applied_migration_names: Vec<String> = Vec::with_capacity(unapplied_migrations.len());

    for unapplied_migration in unapplied_migrations {
        let fut = async {
            let script = unapplied_migration
                .read_migration_script()
                .map_err(ConnectorError::from)?;

            tracing::info!(
                script = script.as_str(),
                "Applying `{}`",
                unapplied_migration.migration_name()
            );

            let migration_id = connector
                .migration_persistence()
                .record_migration_started(unapplied_migration.migration_name(), &script)
                .await?;

            match connector
                .apply_script(unapplied_migration.migration_name(), &script)
                .await
            {
                Ok(()) => {
                    tracing::debug!("Successfully applied the script.");
                    let p = connector.migration_persistence();
                    p.record_successful_step(&migration_id).await?;
                    p.record_migration_finished(&migration_id).await?;
                    applied_migration_names.push(unapplied_migration.migration_name().to_owned());
                    Ok(())
                }
                Err(err) => {
                    tracing::debug!("Failed to apply the script.");

                    let logs = err.to_string();

                    connector
                        .migration_persistence()
                        .record_failed_step(&migration_id, &logs)
                        .await?;

                    Err(err)
                }
            }
        };
        fut.instrument(tracing::info_span!(
            "Applying migration",
            migration_name = unapplied_migration.migration_name(),
        ))
        .await?
    }

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
        let logs = failed_migration
            .logs
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| format!(" with the following logs:\n{s}"))
            .unwrap_or_default();

        writeln!(
            details,
            "The `{name}` migration started at {started_at} failed{logs}",
            name = failed_migration.migration_name,
            started_at = failed_migration.started_at,
        )
        .unwrap();
    }

    Err(CoreError::user_facing(FoundFailedMigrations { details }))
}
