use crate::{destructive_change_checker::DestructiveChangeDiagnostics, ConnectorResult};

/// Apply a single migration step to the connector's database. At this level, we are working with database migrations,
/// i.e. the [associated type on MigrationConnector](trait.MigrationConnector.html#associatedtype.DatabaseMigration).
#[async_trait::async_trait]
pub trait DatabaseMigrationStepApplier<T>: Send + Sync {
    /// Applies the migration to the database. Returns the number of executed steps.
    async fn apply_migration(&self, database_migration: &T) -> ConnectorResult<u32>;

    /// Render steps for the CLI. Each step will contain the raw field.
    fn render_steps_pretty(&self, database_migration: &T) -> ConnectorResult<Vec<PrettyDatabaseMigrationStep>>;

    /// Render the migration to a runnable script.
    fn render_script(&self, database_migration: &T, diagnostics: &DestructiveChangeDiagnostics) -> String;

    /// Apply a migration script to the database. The migration persistence is
    /// managed by the core.
    async fn apply_script(&self, script: &str) -> ConnectorResult<()>;
}

/// A helper struct to serialize a database migration with an additional `raw` field containing the
/// rendered query string for that step.
#[derive(Debug, Clone)]
pub struct PrettyDatabaseMigrationStep {
    /// The raw query string.
    pub raw: String,
}
