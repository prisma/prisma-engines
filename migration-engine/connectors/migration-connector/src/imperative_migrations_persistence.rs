use crate::{checksum, ConnectorError, ConnectorResult};
use serde::Deserialize;

/// A timestamp.
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// Management of imperative migrations state in the database.
#[async_trait::async_trait]
pub trait ImperativeMigrationsPersistence: Send + Sync {
    /// This method is responsible for checking whether the migrations
    /// persistence is initialized.
    ///
    /// If the migration persistence is not present in the target database,
    /// check whether the database schema is empty. If it is, initialize the
    /// migration persistence. If not, return a DatabaseSchemaNotEmpty error unless
    /// we are in the baselining case.
    async fn initialize(&self, baseline: bool) -> ConnectorResult<()>;

    /// Implementation in the connector for the core's MarkMigrationApplied
    /// command. See the docs there. Note that the started_at and finished_at
    /// for the migration should be the same.
    ///
    /// Connectors should implement mark_migration_applied_impl to avoid doing
    /// the checksuming themselves.
    async fn mark_migration_applied(&self, migration_name: &str, script: &str) -> ConnectorResult<String> {
        self.mark_migration_applied_impl(migration_name, script, &checksum(script))
            .await
    }

    /// Implementation in the connector for the core's MarkMigrationApplied
    /// command. See the docs there. Note that the started_at and finished_at
    /// for the migration should be the same.
    async fn mark_migration_applied_impl(
        &self,
        migration_name: &str,
        script: &str,
        checksum: &str,
    ) -> ConnectorResult<String>;

    /// Mark the failed instances of the migration in the persistence as rolled
    /// back, so they will be ignored by the engine in the future.
    async fn mark_migration_rolled_back_by_id(&self, migration_id: &str) -> ConnectorResult<()>;

    /// Record that a migration is about to be applied. Returns the unique
    /// identifier for the migration.
    ///
    /// This is a default method that computes the checkum. Implementors should
    /// implement record_migration_started_impl.
    async fn record_migration_started(&self, migration_name: &str, script: &str) -> ConnectorResult<String> {
        self.record_migration_started_impl(migration_name, script, &checksum(script))
            .await
    }

    /// Record that a migration is about to be applied. Returns the unique
    /// identifier for the migration.
    ///
    /// This is an implementation detail, consumers should use
    /// `record_migration_started()` instead.
    async fn record_migration_started_impl(
        &self,
        migration_name: &str,
        script: &str,
        checksum: &str,
    ) -> ConnectorResult<String>;

    /// Increase the applied_steps_count counter, and append the given logs.
    async fn record_successful_step(&self, id: &str, logs: &str) -> ConnectorResult<()>;

    /// Report logs for a failed migration step. We assume the next steps in the
    /// migration will not be applied, and the error reported.
    async fn record_failed_step(&self, id: &str, logs: &str) -> ConnectorResult<()>;

    /// Record that the migration completed *successfully*. This means
    /// populating the `finished_at` field in the migration record.
    async fn record_migration_finished(&self, id: &str) -> ConnectorResult<()>;

    /// List all applied migrations, ordered by `started_at`. This should fail
    /// hard if the migration persistence is not initialized.
    async fn list_migrations(&self) -> ConnectorResult<Result<Vec<MigrationRecord>, PersistenceNotInitializedError>>;
}

/// Error returned when the persistence is not initialized.
#[derive(Debug)]
pub struct PersistenceNotInitializedError;

impl PersistenceNotInitializedError {
    /// Explicit conversion to a ConnectorError.
    pub fn into_connector_error(self) -> ConnectorError {
        ConnectorError::generic(anyhow::anyhow!(
            "Invariant violation: migration persistence is not initialized."
        ))
    }
}

/// An applied migration, as returned by list_migrations.
#[derive(Debug, PartialEq, Deserialize)]
pub struct MigrationRecord {
    /// A unique, randomly generated identifier.
    pub id: String,
    /// The SHA-256 checksum of the migration script, to detect if it was
    /// edited. It covers only the content of the script, it does not include
    /// timestamp or migration name information.
    pub checksum: String,
    /// The timestamp at which the migration completed *successfully*.
    pub finished_at: Option<Timestamp>,
    /// The name of the migration, i.e. the name of migration directory
    /// containing the migration script.
    pub migration_name: String,
    /// The human-readable log of actions performed by the engine, up to and
    /// including the point where the migration failed, with the relevant error.
    ///
    /// Implementation detail note: a tracing collector with specific events in
    /// the database applier.
    pub logs: String,
    /// If the migration was rolled back, and when.
    pub rolled_back_at: Option<Timestamp>,
    /// The time the migration started being applied.
    pub started_at: Timestamp,
    /// The number of migration steps that were successfully applied.
    pub applied_steps_count: u32,
    /// The whole migration script.
    pub script: String,
}
