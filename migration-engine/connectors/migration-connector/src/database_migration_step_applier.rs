use crate::{destructive_change_checker::DestructiveChangeDiagnostics, ConnectorResult, Migration};

/// Apply a single migration step to the connector's database.
#[async_trait::async_trait]
pub trait DatabaseMigrationStepApplier: Send + Sync {
    /// Applies the migration to the database. Returns the number of executed steps.
    async fn apply_migration(&self, migration: &Migration) -> ConnectorResult<u32>;

    /// Render the migration to a runnable script.
    fn render_script(&self, migration: &Migration, diagnostics: &DestructiveChangeDiagnostics) -> String;

    /// Apply a migration script to the database. The migration persistence is
    /// managed by the core.
    async fn apply_script(&self, migration_name: &str, script: &str) -> ConnectorResult<()>;
}
