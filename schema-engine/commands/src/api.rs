//! The external facing programmatic API to the schema engine.

use crate::{CoreResult, commands};
use json_rpc::types::*;

/// The programmatic, generic, fantastic schema engine API.
#[async_trait::async_trait]
pub trait GenericApi: Send + Sync + 'static {
    /// Return the database version as a string.
    async fn version(&self, params: Option<GetDatabaseVersionInput>) -> CoreResult<String>;

    /// Apply all the unapplied migrations from the migrations folder.
    async fn apply_migrations(&self, input: ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput>;

    /// Create the database referenced by Prisma schema that was used to initialize the connector.
    async fn create_database(&self, params: CreateDatabaseParams) -> CoreResult<CreateDatabaseResult>;

    /// Generate a new migration, based on the provided schema and existing migrations history.
    async fn create_migration(&self, input: CreateMigrationInput) -> CoreResult<CreateMigrationOutput>;

    /// Send a raw command to the database.
    async fn db_execute(&self, params: DbExecuteParams) -> CoreResult<()>;

    /// Debugging method that only panics, for CLI tests.
    async fn debug_panic(&self) -> CoreResult<()>;

    /// Tells the CLI what to do in `migrate dev`.
    async fn dev_diagnostic(&self, input: DevDiagnosticInput) -> CoreResult<DevDiagnosticOutput>;

    /// Create a migration between any two sources of database schemas.
    async fn diff(&self, params: DiffParams) -> CoreResult<DiffResult>;

    /// Drop the database referenced by Prisma schema that was used to initialize the connector.
    async fn drop_database(&self, url: String) -> CoreResult<()>;

    /// Looks at the migrations folder and the database, and returns a bunch of useful information.
    async fn diagnose_migration_history(
        &self,
        input: commands::DiagnoseMigrationHistoryInput,
    ) -> CoreResult<commands::DiagnoseMigrationHistoryOutput>;

    /// Make sure the connection to the database is established and valid.
    /// Connectors can choose to connect lazily, but this method should force
    /// them to connect.
    async fn ensure_connection_validity(
        &self,
        params: EnsureConnectionValidityParams,
    ) -> CoreResult<EnsureConnectionValidityResult>;

    /// Evaluate the consequences of running the next migration we would generate, given the current state of a Prisma schema.
    async fn evaluate_data_loss(&self, input: EvaluateDataLossInput) -> CoreResult<EvaluateDataLossOutput>;

    /// Introspect the database schema.
    async fn introspect(&self, input: IntrospectParams) -> CoreResult<IntrospectResult>;

    /// Introspects a SQL query and returns types information
    async fn introspect_sql(&self, input: IntrospectSqlParams) -> CoreResult<IntrospectSqlResult>;

    /// Mark a migration from the migrations folder as applied, without actually applying it.
    async fn mark_migration_applied(&self, input: MarkMigrationAppliedInput) -> CoreResult<MarkMigrationAppliedOutput>;

    /// Mark a migration as rolled back.
    async fn mark_migration_rolled_back(
        &self,
        input: MarkMigrationRolledBackInput,
    ) -> CoreResult<MarkMigrationRolledBackOutput>;

    /// Reset a database to an empty state (no data, no schema).
    async fn reset(&self, input: ResetInput) -> CoreResult<()>;

    /// The command behind `prisma db push`.
    async fn schema_push(&self, input: SchemaPushInput) -> CoreResult<SchemaPushOutput>;

    /// Dispose of all connectors and gracefully close the underlying database connections.
    /// Failure to call this method must not result in resource leaks or correctness issues,
    /// but may result in connections being closed abruptly without notifying the database.
    /// It is acceptable to not call this method in tests or test harness but it needs to be
    /// called explicitly in the user facing binary to ensure compatibility with PGlite which
    /// can't handle abrupt disconnections.
    async fn dispose(&mut self) -> CoreResult<()>;
}
