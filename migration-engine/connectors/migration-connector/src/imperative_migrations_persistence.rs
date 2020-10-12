use crate::{ConnectorError, ConnectorResult};
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
    /// migration persistence. If not, return a NonEmptyDatabase error.
    async fn initialize(&self) -> ConnectorResult<()>;

    /// Record that a migration is about to be applied. Returns the unique identifier for the migration.
    async fn record_migration_started(&self, migration_name: &str, script: &str) -> ConnectorResult<String>;

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
#[derive(Debug, Deserialize)]
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

impl MigrationRecord {
    /// Is the migration in a failed state?
    pub fn is_failed(&self) -> bool {
        self.finished_at.is_none()
    }
}
