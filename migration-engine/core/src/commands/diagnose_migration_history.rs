use std::path::Path;

use super::MigrationCommand;
use crate::{migration_engine::MigrationEngine, CoreResult};
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
    /// Whether drift between the expected schema and the dev database could be
    /// detected. `None` if the dev database has the expected schema.
    pub drift: Option<DriftDiagnostic>,
    /// The current status of the migration history of the database relative to
    /// migrations directory. `None` if they are in sync and up to date.
    pub history: Option<HistoryDiagnostic>,
    /// The names of the migrations that are currently in a failed state in the
    /// database.
    pub failed_migration_names: Vec<String>,
    /// The names of the migrations for which the checksum of the script in the
    /// migration directory does not match the checksum of the applied migration
    /// in the database.
    pub edited_migration_names: Vec<String>,
}

impl DiagnoseMigrationHistoryOutput {
    /// True if no problem was found
    pub fn is_empty(&self) -> bool {
        self.drift.is_none()
            && self.history.is_none()
            && self.failed_migration_names.is_empty()
            && self.edited_migration_names.is_empty()
    }
}

/// Read the contents of the migrations directory and the migrations table, and
/// returns their relative statuses. At this stage, the migration engine only
/// reads, it does not write to the dev database nor the migrations directory.
pub struct DiagnoseMigrationHistoryCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for DiagnoseMigrationHistoryCommand {
    type Input = DiagnoseMigrationHistoryInput;

    type Output = DiagnoseMigrationHistoryOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let migration_persistence = connector.new_migration_persistence();
        let migration_inferrer = connector.database_migration_inferrer();

        tracing::debug!("Diagnosing migration history");

        // Load the migrations.
        let migrations_from_filesystem =
            migration_connector::list_migrations(&Path::new(&input.migrations_directory_path))?;
        let migrations_from_database = migration_persistence.list_migrations().await?.unwrap_or_default();

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
            drift: diagnostics.drift(),
            history: diagnostics.history(),
            failed_migration_names: diagnostics.failed_migration_names(),
            edited_migration_names: diagnostics.edited_migration_names(),
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

    fn history(&self) -> Option<HistoryDiagnostic> {
        match (self.fs_migrations_not_in_db.len(), self.db_migrations_not_in_fs.len()) {
            (0, 0) => None,
            (_, 0) => Some(HistoryDiagnostic::DatabaseIsBehind {
                unapplied_migration_names: self.fs_migration_names(),
            }),
            (0, _) => Some(HistoryDiagnostic::MigrationsDirectoryIsBehind {
                unpersisted_migration_names: self.db_migration_names(),
            }),
            (_, _) => Some(HistoryDiagnostic::HistoriesDiverge {
                last_common_migration_name: self.fs_migrations_not_in_db.first().and_then(|(idx, _)| {
                    if *idx == 0 {
                        None
                    } else {
                        Some(self.fs_migrations[idx - 1].migration_name().to_owned())
                    }
                }),
                unpersisted_migration_names: self.db_migration_names(),
                unapplied_migration_names: self.fs_migration_names(),
            }),
        }
    }

    fn drift(&self) -> Option<DriftDiagnostic> {
        if self.drift_detected {
            return Some(DriftDiagnostic::DriftDetected);
        }

        if let Some((migration_name, error)) = &self.migration_failed_to_apply {
            return Some(DriftDiagnostic::MigrationFailedToApply {
                migration_name: migration_name.clone(),
                error: error.clone(),
            });
        }

        None
    }
}

/// A diagnostic returned by `diagnoseMigrationHistory` when looking at the
/// database migration history in relation to the migrations directory.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "diagnostic", rename_all = "camelCase")]
pub enum HistoryDiagnostic {
    /// There are migrations in the migrations directory that have not been
    /// applied to the database yet.
    #[serde(rename_all = "camelCase")]
    DatabaseIsBehind {
        /// The names of the migrations.
        unapplied_migration_names: Vec<String>,
    },
    /// Migrations have been applied to the database that are not in the
    /// migrations directory.
    #[serde(rename_all = "camelCase")]
    MigrationsDirectoryIsBehind {
        /// The names of the migrations.
        unpersisted_migration_names: Vec<String>,
    },
    /// The migrations table history and the migrations directory history are
    /// not the same. This currently ignores the ordering of migrations.
    #[serde(rename_all = "camelCase")]
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
}

/// A diagnostic returned by `diagnoseMigrationHistory` when trying to determine
/// whether the development database has the expected schema at its stage in
/// history.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "diagnostic", rename_all = "camelCase")]
pub enum DriftDiagnostic {
    /// The database schema of the current database does not match what would be
    /// expected at its stage in the migration history.
    DriftDetected,
    /// When a migration fails to apply cleanly to a temporary database.
    #[serde(rename_all = "camelCase")]
    MigrationFailedToApply {
        /// The name of the migration that failed.
        migration_name: String,
        /// The full error.
        error: String,
    },
}
