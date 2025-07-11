use crate::CoreResult;
use commands::{DiagnoseMigrationHistoryOutput, DriftDiagnostic, MigrationSchemaCache};
pub use json_rpc::types::{DiagnoseMigrationHistoryInput, HistoryDiagnostic};
use schema_connector::{
    migrations_directory::{error_on_changed_provider, list_migrations, MigrationDirectory},
    ConnectorError, MigrationRecord, Namespaces, PersistenceNotInitializedError, SchemaConnector, SchemaFilter,
};

/// Read the contents of the migrations directory and the migrations table, and
/// returns their relative statuses. At this stage, the schema engine only
/// reads, it does not write to the dev database nor the migrations directory.
pub async fn diagnose_migration_history_cli(
    input: DiagnoseMigrationHistoryInput,
    namespaces: Option<Namespaces>,
    connector: &mut dyn SchemaConnector,
    migration_schema_cache: &mut MigrationSchemaCache,
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
            let dialect = connector.schema_dialect();
            let filter = SchemaFilter::from_filter_and_namespaces(input.schema_filter, namespaces.clone());
            let from = migration_schema_cache
                .get_or_insert(&applied_migrations, || async {
                    connector.schema_from_migrations(&applied_migrations, &filter).await
                })
                .await;

            let to = connector.schema_from_database(namespaces.clone()).await;
            let drift = match from
                .and_then(|from| to.map(|to| dialect.diff(from, to, &filter)))
                .map(|mig| {
                    if dialect.migration_is_empty(&mig) {
                        None
                    } else {
                        Some(mig)
                    }
                }) {
                Ok(Some(drift)) => Some(DriftDiagnostic::DriftDetected {
                    summary: dialect.migration_summary(&drift),
                }),
                Err(error) => Some(DriftDiagnostic::MigrationFailedToApply { error }),
                _ => None,
            };

            let error_in_unapplied_migration = if !matches!(drift, Some(DriftDiagnostic::MigrationFailedToApply { .. }))
            {
                connector
                    .validate_migrations(&migrations_from_filesystem, &filter)
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
