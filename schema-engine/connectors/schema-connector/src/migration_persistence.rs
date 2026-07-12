use crate::{BoxFuture, ConnectorError, ConnectorResult, Namespaces, SchemaFilter, checksum};

/// A timestamp.
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// Management of imperative migrations state in the database.
pub trait MigrationPersistence: Send + Sync {
    /// Initialize the migration persistence without checking the database first.
    fn baseline_initialize(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// This method is responsible for checking whether the migrations
    /// persistence is initialized.
    ///
    /// If the migration persistence is not present in the target database,
    /// check whether the database schema is empty. If it is, initialize the
    /// migration persistence. If not, return a DatabaseSchemaNotEmpty error.
    fn initialize(
        &mut self,
        namespaces: Option<Namespaces>,
        filters: SchemaFilter,
    ) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Implementation in the connector for the core's MarkMigrationApplied
    /// command. See the docs there. Note that the started_at and finished_at
    /// for the migration should be the same.
    ///
    /// Connectors should implement mark_migration_applied_impl to avoid doing
    /// the checksuming themselves.
    fn mark_migration_applied<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<String>> {
        Box::pin(async move {
            self.mark_migration_applied_impl(migration_name, &checksum::render_checksum(script))
                .await
        })
    }

    /// Implementation in the connector for the core's MarkMigrationApplied
    /// command. See the docs there. Note that the started_at and finished_at
    /// for the migration should be the same.
    fn mark_migration_applied_impl<'a>(
        &'a mut self,
        migration_name: &'a str,
        checksum: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<String>>;

    /// Mark the failed instances of the migration in the persistence as rolled
    /// back, so they will be ignored by the engine in the future.
    fn mark_migration_rolled_back_by_id<'a>(&'a mut self, migration_id: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Record that a migration is about to be applied. Returns the unique
    /// identifier for the migration.
    ///
    /// This is a default method that computes the checksum. Implementors should
    /// implement record_migration_started_impl.
    fn record_migration_started<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<String>> {
        Box::pin(async move {
            self.record_migration_started_impl(migration_name, &checksum::render_checksum(script))
                .await
        })
    }

    /// Record that a migration is about to be applied. Returns the unique
    /// identifier for the migration.
    ///
    /// This is an implementation detail, consumers should use
    /// `record_migration_started()` instead.
    fn record_migration_started_impl<'a>(
        &'a mut self,
        migration_name: &'a str,
        checksum: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<String>>;

    /// Increase the applied_steps_count counter.
    fn record_successful_step<'a>(&'a mut self, id: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Report logs for a failed migration step. We assume the next steps in the
    /// migration will not be applied, and the error reported.
    fn record_failed_step<'a>(&'a mut self, id: &'a str, logs: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Record that the migration completed *successfully*. This means
    /// populating the `finished_at` field in the migration record.
    fn record_migration_finished<'a>(&'a mut self, id: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;

    /// List all applied migrations, ordered by `started_at`. This should fail
    /// with a PersistenceNotInitializedError when the migration persistence is
    /// not initialized.
    fn list_migrations(
        &mut self,
    ) -> BoxFuture<'_, ConnectorResult<Result<Vec<MigrationRecord>, PersistenceNotInitializedError>>>;
}

/// Error returned when the persistence is not initialized.
#[derive(Debug)]
pub struct PersistenceNotInitializedError;

impl PersistenceNotInitializedError {
    /// Explicit conversion to a ConnectorError.
    pub fn into_connector_error(self) -> ConnectorError {
        ConnectorError::from_msg("Invariant violation: migration persistence is not initialized.".into())
    }
}

/// An applied migration, as returned by list_migrations.
#[derive(Debug)]
pub struct MigrationRecord {
    /// A unique, randomly generated identifier.
    pub id: String,
    /// The SHA-256 checksum of the migration script, to detect if it was
    /// edited. It covers only the content of the script file, it does not
    /// include timestamp or migration name information.
    pub checksum: String,
    /// The timestamp at which the migration completed *successfully*.
    pub finished_at: Option<Timestamp>,
    /// The name of the migration, i.e. the name of migration directory
    /// containing the migration script.
    pub migration_name: String,
    /// The human-readable log of actions performed by the engine, up to and
    /// including the point where the migration failed, with the relevant error.
    pub logs: Option<String>,
    /// If the migration was rolled back, and when.
    pub rolled_back_at: Option<Timestamp>,
    /// The time the migration started being applied.
    pub started_at: Timestamp,
    /// The number of migration steps that were successfully applied.
    pub applied_steps_count: u32,
}
