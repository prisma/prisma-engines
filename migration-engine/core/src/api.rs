//! The external facing programmatic API to the migration engine.

use crate::{commands::*, CoreResult};
use migration_connector::{migrations_directory, MigrationConnector};
use std::path::Path;
use tracing_futures::Instrument;

/// The programmatic, generic, fantastic migration engine API.
#[async_trait::async_trait]
pub trait GenericApi: Send + Sync + 'static {
    /// Return the database version as a string.
    async fn version(&self) -> CoreResult<String>;

    /// Apply all the unapplied migrations from the migrations folder.
    async fn apply_migrations(&self, input: &ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput>;

    /// Generate a new migration, based on the provided schema and existing migrations history.
    async fn create_migration(&self, input: &CreateMigrationInput) -> CoreResult<CreateMigrationOutput>;

    /// Debugging method that only panics, for CLI tests.
    async fn debug_panic(&self) -> CoreResult<()>;

    /// Tells the CLI what to do in `migrate dev`.
    async fn dev_diagnostic(&self, input: &DevDiagnosticInput) -> CoreResult<DevDiagnosticOutput>;

    /// Looks at the migrations folder and the database, and returns a bunch of useful information.
    async fn diagnose_migration_history(
        &self,
        input: &DiagnoseMigrationHistoryInput,
    ) -> CoreResult<DiagnoseMigrationHistoryOutput>;

    /// Evaluate the consequences of running the next migration we would generate, given the current state of a Prisma schema.
    async fn evaluate_data_loss(&self, input: &EvaluateDataLossInput) -> CoreResult<EvaluateDataLossOutput>;

    /// List the migration directories.
    async fn list_migration_directories(
        &self,
        input: &ListMigrationDirectoriesInput,
    ) -> CoreResult<ListMigrationDirectoriesOutput>;

    /// Mark a migration from the migrations folder as applied, without actually applying it.
    async fn mark_migration_applied(&self, input: &MarkMigrationAppliedInput)
        -> CoreResult<MarkMigrationAppliedOutput>;

    /// Mark a migration as rolled back.
    async fn mark_migration_rolled_back(
        &self,
        input: &MarkMigrationRolledBackInput,
    ) -> CoreResult<MarkMigrationRolledBackOutput>;

    /// Prepare to create a migration.
    async fn plan_migration(&self) -> CoreResult<()>;

    /// Reset a database to an empty state (no data, no schema).
    async fn reset(&self) -> CoreResult<()>;

    /// The command behind `prisma db push`.
    async fn schema_push(&self, input: &SchemaPushInput) -> CoreResult<SchemaPushOutput>;
}

#[async_trait::async_trait]
impl<C: MigrationConnector> GenericApi for C {
    async fn version(&self) -> CoreResult<String> {
        Ok(self.version().await?)
    }

    async fn apply_migrations(&self, input: &ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput> {
        apply_migrations(input, self)
            .instrument(tracing::info_span!("ApplyMigrations"))
            .await
    }

    async fn create_migration(&self, input: &CreateMigrationInput) -> CoreResult<CreateMigrationOutput> {
        create_migration(input, self)
            .instrument(tracing::info_span!(
                "CreateMigration",
                migration_name = input.migration_name.as_str(),
                draft = input.draft,
            ))
            .await
    }

    async fn debug_panic(&self) -> CoreResult<()> {
        panic!("This is the debugPanic artificial panic")
    }

    async fn dev_diagnostic(&self, input: &DevDiagnosticInput) -> CoreResult<DevDiagnosticOutput> {
        dev_diagnostic(input, self)
            .instrument(tracing::info_span!("DevDiagnostic"))
            .await
    }

    async fn diagnose_migration_history(
        &self,
        input: &DiagnoseMigrationHistoryInput,
    ) -> CoreResult<DiagnoseMigrationHistoryOutput> {
        diagnose_migration_history(input, self)
            .instrument(tracing::info_span!("DiagnoseMigrationHistory"))
            .await
    }

    async fn evaluate_data_loss(&self, input: &EvaluateDataLossInput) -> CoreResult<EvaluateDataLossOutput> {
        evaluate_data_loss(input, self)
            .instrument(tracing::info_span!("EvaluateDataLoss"))
            .await
    }

    async fn list_migration_directories(
        &self,
        input: &ListMigrationDirectoriesInput,
    ) -> CoreResult<ListMigrationDirectoriesOutput> {
        let migrations_from_filesystem =
            migrations_directory::list_migrations(Path::new(&input.migrations_directory_path))?;

        let migrations = migrations_from_filesystem
            .iter()
            .map(|migration| migration.migration_name().to_string())
            .collect();

        Ok(ListMigrationDirectoriesOutput { migrations })
    }

    async fn mark_migration_applied(
        &self,
        input: &MarkMigrationAppliedInput,
    ) -> CoreResult<MarkMigrationAppliedOutput> {
        mark_migration_applied(input, self)
            .instrument(tracing::info_span!(
                "MarkMigrationApplied",
                migration_name = input.migration_name.as_str()
            ))
            .await
    }

    async fn mark_migration_rolled_back(
        &self,
        input: &MarkMigrationRolledBackInput,
    ) -> CoreResult<MarkMigrationRolledBackOutput> {
        mark_migration_rolled_back(input, self)
            .instrument(tracing::info_span!(
                "MarkMigrationRolledBack",
                migration_name = input.migration_name.as_str()
            ))
            .await
    }

    async fn plan_migration(&self) -> CoreResult<()> {
        unreachable!("PlanMigration command")
    }

    async fn reset(&self) -> CoreResult<()> {
        tracing::debug!("Resetting the database.");

        Ok(MigrationConnector::reset(self)
            .instrument(tracing::info_span!("Reset"))
            .await?)
    }

    async fn schema_push(&self, input: &SchemaPushInput) -> CoreResult<SchemaPushOutput> {
        schema_push(input, self)
            .instrument(tracing::info_span!("SchemaPush"))
            .await
    }
}
