use std::path::Path;

use super::MigrationCommand;
use crate::migration_engine::MigrationEngine;
use migration_connector::{ErrorKind, MigrationDirectory, MigrationRecord};
use serde::{Deserialize, Serialize};

/// The input to the `DiagnoseMigrationHistory` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseMigrationHistoryInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
}

/// The output of the `DiagnoseMigrationHistory` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseMigrationHistoryOutput {
    /// Issues detected after examining the migrations history.
    pub history_problems: Vec<HistoryDiagnostic>,
}

/// Read the contents of the migrations directory and the migrations table, and
/// returns their relative statuses. At this stage, the migration engine only
/// reads, it does not write to the dev database nor the migrations directory.
pub struct DiagnoseMigrationHistoryCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for DiagnoseMigrationHistoryCommand {
    type Input = DiagnoseMigrationHistoryInput;

    type Output = DiagnoseMigrationHistoryOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> super::CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let migration_persistence = connector.new_migration_persistence();
        let migration_inferrer = connector.database_migration_inferrer();

        // Load the migrations.
        let migrations_from_filesystem =
            migration_connector::list_migrations(&Path::new(&input.migrations_directory_path))
                .expect("Failed to list migrations");
        let migrations_from_database = migration_persistence.list_migrations().await?;

        let mut diagnostics = Diagnostics::new(&migrations_from_filesystem);

        // Check filesystem history against database history.
        for (index, fs_migration) in migrations_from_filesystem.iter().enumerate() {
            let corresponding_db_migration = migrations_from_database
                .iter()
                .find(|db_migration| db_migration.migration_name == fs_migration.migration_name());

            match corresponding_db_migration {
                Some(db_migration)
                    if !fs_migration
                        .matches_checksum(&db_migration.checksum)
                        .expect("Failed to read migration script") =>
                {
                    diagnostics.edited_migrations.push(db_migration);
                }
                Some(_) => (),
                None => diagnostics.fs_migrations_not_in_db.push((index, fs_migration)),
            }
        }

        for (index, db_migration) in migrations_from_database.iter().enumerate() {
            let corresponding_fs_migration = migrations_from_filesystem
                .iter()
                .find(|fs_migration| db_migration.migration_name == fs_migration.migration_name());

            if corresponding_fs_migration.is_none() {
                diagnostics.db_migrations_not_in_fs.push((index, db_migration))
            }
        }

        // Detect drift
        let applied_migrations: Vec<_> = migrations_from_filesystem
            .iter()
            .filter(|fs_migration| {
                migrations_from_database.iter().any(|db_migration| {
                    db_migration.migration_name == fs_migration.migration_name() && !db_migration.is_failed()
                })
            })
            .cloned()
            .collect();
        diagnostics.drift_detected = match migration_inferrer.detect_drift(&applied_migrations).await {
            Ok(drift_detected) => drift_detected,
            Err(err) => match &err.kind {
                ErrorKind::MigrationFailedToApply { migration_name, error } => {
                    diagnostics.migration_failed_to_apply = Some((migration_name.clone(), error.to_string()));
                    false
                }
                _ => return Err(err.into()),
            },
        };

        Ok(DiagnoseMigrationHistoryOutput {
            history_problems: diagnostics.into(),
        })
    }
}

#[derive(Debug)]
struct Diagnostics<'a> {
    fs_migrations_not_in_db: Vec<(usize, &'a MigrationDirectory)>,
    db_migrations_not_in_fs: Vec<(usize, &'a MigrationRecord)>,
    edited_migrations: Vec<&'a MigrationRecord>,
    failed_migrations: Vec<&'a MigrationRecord>,
    drift_detected: bool,
    /// Name and error.
    migration_failed_to_apply: Option<(String, String)>,
    fs_migrations: &'a [MigrationDirectory],
}

impl<'a> Diagnostics<'a> {
    fn new(fs_migrations: &'a [MigrationDirectory]) -> Self {
        Diagnostics {
            fs_migrations_not_in_db: Vec::new(),
            db_migrations_not_in_fs: Vec::new(),
            edited_migrations: Vec::new(),
            failed_migrations: Vec::new(),
            drift_detected: false,
            migration_failed_to_apply: None,
            fs_migrations,
        }
    }

    fn db_migration_names(&self) -> Vec<String> {
        self.db_migrations_not_in_fs
            .iter()
            .map(|(_, migration)| migration.migration_name.clone())
            .collect()
    }

    fn edited_migration_names(&self) -> Vec<String> {
        self.edited_migrations
            .iter()
            .map(|migration| migration.migration_name.clone())
            .collect()
    }

    fn failed_migration_names(&self) -> Vec<String> {
        self.failed_migrations
            .iter()
            .map(|migration| migration.migration_name.clone())
            .collect()
    }

    fn fs_migration_names(&self) -> Vec<String> {
        self.fs_migrations_not_in_db
            .iter()
            .map(|(_, migration)| migration.migration_name().to_owned())
            .collect()
    }
}

impl From<Diagnostics<'_>> for Vec<HistoryDiagnostic> {
    fn from(diagnostics: Diagnostics<'_>) -> Self {
        let mut problems = Vec::new();

        match (
            diagnostics.fs_migrations_not_in_db.len(),
            diagnostics.db_migrations_not_in_fs.len(),
        ) {
            (0, 0) => (),
            (_, 0) => problems.push(HistoryDiagnostic::DatabaseIsBehind {
                unapplied_migration_names: diagnostics.fs_migration_names(),
            }),
            (0, _) => problems.push(HistoryDiagnostic::MigrationsDirectoryIsBehind {
                unpersisted_migration_names: diagnostics.db_migration_names(),
            }),
            (_, _) => problems.push(HistoryDiagnostic::HistoriesDiverge {
                last_common_migration_name: diagnostics.fs_migrations_not_in_db.first().and_then(|(idx, _)| {
                    if *idx == 0 {
                        None
                    } else {
                        Some(diagnostics.fs_migrations[idx - 1].migration_name().to_owned())
                    }
                }),
                unpersisted_migration_names: diagnostics.db_migration_names(),
                unapplied_migration_names: diagnostics.fs_migration_names(),
            }),
        }

        if !diagnostics.edited_migrations.is_empty() {
            problems.push(HistoryDiagnostic::MigrationsEdited {
                edited_migration_names: diagnostics.edited_migration_names(),
            })
        }

        if !diagnostics.failed_migrations.is_empty() {
            problems.push(HistoryDiagnostic::MigrationsFailed {
                failed_migration_names: diagnostics.failed_migration_names(),
            })
        }

        if diagnostics.drift_detected {
            problems.push(HistoryDiagnostic::DriftDetected)
        }

        if let Some((migration_name, error)) = diagnostics.migration_failed_to_apply {
            problems.push(HistoryDiagnostic::MigrationFailedToApply { migration_name, error });
        }

        problems
    }
}

/// A diagnostic returned by `diagnoseMigrationHistory`.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "diagnostic", rename_all = "camelCase")]
pub enum HistoryDiagnostic {
    /// Migration scripts were edited.
    MigrationsEdited {
        /// The names of the migration directories in the migrations directory.
        edited_migration_names: Vec<String>,
    },
    /// There are migrations in the database that are not completely applied/failed to apply.
    MigrationsFailed {
        /// The names of the migrations.
        failed_migration_names: Vec<String>,
    },
    /// There are migrations in the migrations directory that have not been applied to the database yet.
    DatabaseIsBehind {
        /// The names of the migrations.
        unapplied_migration_names: Vec<String>,
    },
    /// Migrations have been applied to the database that are not in the migrations directory.
    MigrationsDirectoryIsBehind {
        /// The names of the migrations.
        unpersisted_migration_names: Vec<String>,
    },
    /// The migrations table history and the migrations directory history are
    /// not the same. This currently ignores the ordering of migrations.
    HistoriesDiverge {
        /// The last migration that is present both in the migrations directory
        /// and the migrations table.
        last_common_migration_name: Option<String>,
        /// The names of the migrations that are present in the migrations table
        /// but not in the migrations directory.
        unpersisted_migration_names: Vec<String>,
        /// The names of the migrations that are present in the migrations
        /// directory but have not been applied to the database.
        unapplied_migration_names: Vec<String>,
    },
    /// The database schema of the current database does not match what would be
    /// expected at its stage in the migration history.
    DriftDetected,
    /// When a migration fails to apply to a temporary database.
    MigrationFailedToApply {
        /// The name of the migration that failed.
        migration_name: String,
        /// The full error.
        error: String,
    },
}
