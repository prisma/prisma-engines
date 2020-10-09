use super::MigrationCommand;
use crate::{migration_engine::MigrationEngine, CoreError, CoreResult};
use migration_connector::{ConnectorError, MigrationDirectory, MigrationRecord};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The input to the `ApplyMigrations` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMigrationsInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
}

/// The output of the `ApplyMigrations` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMigrationsOutput {
    /// The names of the migrations that were just applied. Empty if no migration was applied.
    pub applied_migration_names: Vec<String>,
}

/// Read the contents of the migrations directory and the migrations table, and
/// returns their relative statuses. At this stage, the migration engine only
/// reads, it does not write to the dev database nor the migrations directory.
pub struct ApplyMigrationsCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for ApplyMigrationsCommand {
    type Input = ApplyMigrationsInput;

    type Output = ApplyMigrationsOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let applier = connector.database_migration_step_applier();
        let migration_persistence = connector.new_migration_persistence();

        let migrations_from_filesystem =
            migration_connector::list_migrations(&Path::new(&input.migrations_directory_path))?;
        let migrations_from_database = migration_persistence.list_migrations().await?;

        diagnose_migration_history(&migrations_from_database, &migrations_from_filesystem)?;

        // We are now on the Happy Path™.
        tracing::debug!("Migration history is OK, applying unapplied migrations.");
        let unapplied_migrations: Vec<&MigrationDirectory> = migrations_from_filesystem
            .iter()
            .filter(|fs_migration| {
                !migrations_from_database
                    .iter()
                    .any(|db_migration| fs_migration.migration_name() == db_migration.migration_name)
            })
            .collect();

        let mut applied_migration_names: Vec<String> = Vec::new();

        for unapplied_migration in unapplied_migrations {
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

            match applier.apply_script(&script).await {
                Ok(()) => {
                    tracing::debug!("Successfully applied the script.");
                    migration_persistence
                        .record_successful_step(&migration_id, &script)
                        .await?;
                    migration_persistence.record_migration_finished(&migration_id).await?;
                    applied_migration_names.push(unapplied_migration.migration_name().to_owned());
                }
                Err(err) => {
                    tracing::debug!("Failed to apply the script.");

                    let logs = format!("script:\n{}\n\nerror:\n{}", script, err);

                    migration_persistence.record_failed_step(&migration_id, &logs).await?;

                    return Err(err.into()); // todo: give more context
                }
            }
        }

        Ok(ApplyMigrationsOutput {
            applied_migration_names,
        })
    }
}

fn diagnose_migration_history(
    migrations_from_database: &[MigrationRecord],
    migrations_from_filesystem: &[MigrationDirectory],
) -> CoreResult<()> {
    tracing::debug!("Running diagnostics.");

    let mut history_problems = HistoryProblems::default();

    let mut failed_migrations = migrations_from_database
        .iter()
        .filter(|migration| migration.is_failed())
        .peekable();

    if failed_migrations.peek().is_some() {
        history_problems
            .failed_migrations
            .extend(failed_migrations.map(|failed_migration| {
                format!(
                    "The `{name}` migration started at {started_at} failed with the following logs:\n{logs}",
                    name = failed_migration.migration_name,
                    started_at = failed_migration.started_at,
                    logs = failed_migration.logs
                )
            }))
    }

    let mut edited_migrations = migrations_from_database
        .iter()
        .filter(|db_migration| {
            migrations_from_filesystem.iter().any(|fs_migration| {
                fs_migration.migration_name() == db_migration.migration_name
                    && !fs_migration
                        .matches_checksum(&db_migration.checksum)
                        .expect("Failed to read migration script to match checksum.")
            })
        })
        .peekable();

    if edited_migrations.peek().is_some() {
        let error_lines = edited_migrations.map(|db_migration| {
            let diagnostic = match db_migration.finished_at {
                Some(finished_at) => format!("and finished at {finished_at}.", finished_at = finished_at),
                None => "but failed.".to_string(),
            };

            format!(
                "- `{migration_name}, started at {started_at} {diagnostic}`",
                started_at = db_migration.started_at,
                migration_name = db_migration.migration_name,
                diagnostic = diagnostic,
            )
        });

        history_problems.edited_migrations = Some(format!(
            "The following migrations scripts are different from those that were applied to the database:\n{:?}",
            error_lines.collect::<Vec<String>>(),
        ));
    }

    history_problems.to_result()
}

#[derive(Default, Debug)]
struct HistoryProblems {
    failed_migrations: Vec<String>,
    edited_migrations: Option<String>,
}

impl HistoryProblems {
    fn to_result(&self) -> CoreResult<()> {
        if self.failed_migrations.is_empty() && self.edited_migrations.is_none() {
            return Ok(());
        }

        let mut error = String::with_capacity(
            self.failed_migrations.iter().map(String::len).sum::<usize>()
                + self.edited_migrations.as_ref().map(String::len).unwrap_or(0),
        );

        for failed_migration in &self.failed_migrations {
            error.push_str(&failed_migration);
            error.push('\n');
        }

        if let Some(edited_migrations) = &self.edited_migrations {
            error.push_str(edited_migrations)
        }

        Err(CoreError::Generic(anyhow::anyhow!("{}", error)))
    }
}
