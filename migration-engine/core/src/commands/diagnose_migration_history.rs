use std::path::Path;

use super::MigrationCommand;
use crate::{migration_engine::MigrationEngine, CoreResult};
use migration_connector::{MigrationDirectory, MigrationRecord, PersistenceNotInitializedError};
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
    /// An optional error encountered when applying a migration that is not
    /// applied in the main database to the shadow database. We do this to
    /// validate that unapplied migrations are at least minimally valid.
    pub error_in_unapplied_migration: Option<user_facing_errors::Error>,
    /// Is the migrations table initialized in the database.
    pub has_migrations_table: bool,
}

impl DiagnoseMigrationHistoryOutput {
    /// True if no problem was found
    pub fn is_empty(&self) -> bool {
        matches!(
            self,
            DiagnoseMigrationHistoryOutput {
                drift,
                history,
                has_migrations_table: _,
                failed_migration_names,
                edited_migration_names,
            } if drift.is_none() && history.is_none() && failed_migration_names.is_empty() && edited_migration_names.is_empty()
        )
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
        let (migrations_from_database, has_migrations_table) = match migration_persistence.list_migrations().await? {
            Ok(migrations) => (migrations, true),
            Err(PersistenceNotInitializedError {}) => (vec![], false),
        };

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

            if db_migration.finished_at.is_none() && db_migration.rolled_back_at.is_none() {
                diagnostics.failed_migrations.push(db_migration);
            }

            if corresponding_fs_migration.is_none() {
                diagnostics.db_migrations_not_in_fs.push((index, db_migration))
            }
        }

        // Detect drift
        let applied_migrations: Vec<_> = migrations_from_filesystem
            .iter()
            .filter(|fs_migration| {
                migrations_from_database
                    .iter()
                    .filter(|db_migration| db_migration.finished_at.is_some() && db_migration.rolled_back_at.is_none())
                    .any(|db_migration| db_migration.migration_name == fs_migration.migration_name())
            })
            .cloned()
            .collect();

        let drift = match migration_inferrer.calculate_drift(&applied_migrations).await {
            Ok(Some(rollback)) => Some(DriftDiagnostic::DriftDetected { rollback }),
            Err(error) => Some(DriftDiagnostic::MigrationFailedToApply {
                error: error.to_user_facing(),
            }),
            _ => None,
        };

        let error_in_unapplied_migration = if !matches!(drift, Some(DriftDiagnostic::MigrationFailedToApply { .. })) {
            migration_inferrer
                .validate_migrations(&migrations_from_filesystem)
                .await
                .err()
                .map(|connector_error| connector_error.to_user_facing())
        } else {
            None
        };

        Ok(DiagnoseMigrationHistoryOutput {
            drift,
            history: diagnostics.history(),
            failed_migration_names: diagnostics.failed_migration_names(),
            edited_migration_names: diagnostics.edited_migration_names(),
            error_in_unapplied_migration,
            has_migrations_table,
        })
    }
}

#[derive(Debug)]
struct Diagnostics<'a> {
    fs_migrations_not_in_db: Vec<(usize, &'a MigrationDirectory)>,
    db_migrations_not_in_fs: Vec<(usize, &'a MigrationRecord)>,
    edited_migrations: Vec<&'a MigrationRecord>,
    failed_migrations: Vec<&'a MigrationRecord>,
    fs_migrations: &'a [MigrationDirectory],
}

impl<'a> Diagnostics<'a> {
    fn new(fs_migrations: &'a [MigrationDirectory]) -> Self {
        Diagnostics {
            fs_migrations_not_in_db: Vec::new(),
            db_migrations_not_in_fs: Vec::new(),
            edited_migrations: Vec::new(),
            failed_migrations: Vec::new(),
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
    DriftDetected {
        /// A database script to correct the drift by reverting to the expected schema.
        rollback: String,
    },
    /// When a migration fails to apply cleanly to a temporary database.
    #[serde(rename_all = "camelCase")]
    MigrationFailedToApply {
        /// The full error.
        error: user_facing_errors::Error,
    },
}

impl DriftDiagnostic {
    /// For tests.
    pub fn unwrap_drift_detected(self) -> String {
        match self {
            DriftDiagnostic::DriftDetected { rollback } => rollback,
            other => panic!("unwrap_drift_detected on {:?}", other),
        }
    }
}
