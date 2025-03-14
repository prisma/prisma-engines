use crate::CoreResult;
use json_rpc::types::MigrationList;
use schema_connector::{
    migrations_directory::{error_on_changed_provider, list_migrations, MigrationDirectory},
    ConnectorError, DiffTarget, MigrationRecord, Namespaces, PersistenceNotInitializedError, SchemaConnector,
};
use serde::{Deserialize, Serialize};

/// The input to the `DiagnoseMigrationHistory` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseMigrationHistoryInput {
    /// The list of migrations, already loaded from disk.
    pub migrations_list: MigrationList,
    /// Whether creating shadow/temporary databases is allowed.
    pub opt_in_to_shadow_database: bool,
}

/// The output of the `DiagnoseMigrationHistory` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseMigrationHistoryOutput {
    /// Whether drift between the expected schema and the dev database could be
    /// detected. `None` if the dev database has the expected schema.
    #[serde(skip)]
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
    #[serde(skip)]
    pub error_in_unapplied_migration: Option<ConnectorError>,
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
                error_in_unapplied_migration,
            } if drift.is_none() && history.is_none() && failed_migration_names.is_empty() && edited_migration_names.is_empty() && error_in_unapplied_migration.is_none()
        )
    }
}

/// Read the contents of the migrations directory and the migrations table, and
/// returns their relative statuses. At this stage, the schema engine only
/// reads, it does not write to the dev database nor the migrations directory.
pub async fn diagnose_migration_history(
    input: DiagnoseMigrationHistoryInput,
    namespaces: Option<Namespaces>,
    connector: &mut dyn SchemaConnector,
) -> CoreResult<DiagnoseMigrationHistoryOutput> {
    tracing::debug!("Diagnosing migration history");

    error_on_changed_provider(&input.migrations_list.lockfile, connector.connector_type())?;
    let migrations_from_filesystem = list_migrations(input.migrations_list.migration_directories);

    let (migrations_from_database, has_migrations_table) =
        match connector.migration_persistence().list_migrations().await? {
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
                    .map_err(ConnectorError::from)? =>
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

    let (drift, error_in_unapplied_migration) = {
        if input.opt_in_to_shadow_database {
            // TODO(MultiSchema): this should probably fill the following namespaces from the CLI since there is
            // no schema to grab the namespaces off, in the case of MultiSchema.
            let from = connector
                .database_schema_from_diff_target(DiffTarget::Migrations(&applied_migrations), None, namespaces.clone())
                .await;
            let to = connector
                .database_schema_from_diff_target(DiffTarget::Database, None, namespaces.clone())
                .await;
            let drift = match from.and_then(|from| to.map(|to| connector.diff(from, to))).map(|mig| {
                if connector.migration_is_empty(&mig) {
                    None
                } else {
                    Some(mig)
                }
            }) {
                Ok(Some(drift)) => Some(DriftDiagnostic::DriftDetected {
                    summary: connector.migration_summary(&drift),
                }),
                Err(error) => Some(DriftDiagnostic::MigrationFailedToApply { error }),
                _ => None,
            };

            let error_in_unapplied_migration = if !matches!(drift, Some(DriftDiagnostic::MigrationFailedToApply { .. }))
            {
                // TODO(MultiSchema): Not entirely sure passing no namespaces here is correct. Probably should
                // also grab this as a CLI argument.
                connector
                    .validate_migrations(&migrations_from_filesystem, namespaces)
                    .await
                    .err()
            } else {
                None
            };

            (drift, error_in_unapplied_migration)
        } else {
            (None, None)
        }
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
#[derive(Debug)]
pub enum DriftDiagnostic {
    /// The database schema of the current database does not match what would be
    /// expected at its stage in the migration history.
    DriftDetected {
        /// The human-readable contents of the drift.
        summary: String,
    },
    /// When a migration fails to apply cleanly to a shadow database.
    MigrationFailedToApply {
        /// The full error.
        error: ConnectorError,
    },
}

impl DriftDiagnostic {
    /// For tests.
    pub fn unwrap_drift_detected(self) -> String {
        match self {
            DriftDiagnostic::DriftDetected { summary } => summary,
            other => panic!("unwrap_drift_detected on {other:?}"),
        }
    }
}
