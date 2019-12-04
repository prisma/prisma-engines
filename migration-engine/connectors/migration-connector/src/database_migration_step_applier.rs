use crate::*;

/// Apply a single migration step to the connector's database. At this level, we are working with database migrations,
/// i.e. the [associated type on MigrationConnector](trait.MigrationConnector.html#associatedtype.DatabaseMigration).
#[async_trait::async_trait]
pub trait DatabaseMigrationStepApplier<T>: Send + Sync + 'static {
    /// Applies the step to the database
    /// Returns true to signal to the caller that there are more steps to apply.
    async fn apply_step(&self, database_migration: &T, step: usize) -> ConnectorResult<bool>;

    /// Applies the step to the database.
    /// Returns true to signal to the caller that there are more steps to unapply.
    async fn unapply_step(&self, database_migration: &T, step: usize) -> ConnectorResult<bool>;

    /// Render steps for the CLI. Each step will contain the raw field.
    fn render_steps_pretty(&self, database_migration: &T) -> ConnectorResult<Vec<serde_json::Value>>;
}
