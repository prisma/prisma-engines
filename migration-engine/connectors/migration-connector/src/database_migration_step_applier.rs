use crate::*;

/// Apply a single migration step to the connector's database. At this level, we are working with database migrations,
/// i.e. the [associated type on MigrationConnector](trait.MigrationConnector.html#associatedtype.DatabaseMigration).
pub trait DatabaseMigrationStepApplier<T>: Send + Sync + 'static {
    // applies the step to the database
    // returns true to signal to the caller that there are more steps to apply
    fn apply_step(&self, database_migration: &T, step: usize) -> ConnectorResult<bool>;

    // applies the step to the database
    // returns true to signal to the caller that there are more steps to unapply
    fn unapply_step(&self, database_migration: &T, step: usize) -> ConnectorResult<bool>;

    // render steps for the CLI. It will contain the raw field
    fn render_steps_pretty(&self, database_migration: &T) -> ConnectorResult<serde_json::Value>;
}
