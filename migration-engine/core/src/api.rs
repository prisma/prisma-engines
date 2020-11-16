mod error_rendering;
mod rpc;

pub use rpc::*;

use crate::{commands::*, migration_engine::MigrationEngine, CoreResult};
use migration_connector::{DatabaseMigrationMarker, MigrationConnector};
use tracing_futures::Instrument;

pub struct MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + 'static,
{
    engine: MigrationEngine<C, D>,
}

impl<C, D> MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    pub async fn new(connector: C) -> CoreResult<Self> {
        let engine = MigrationEngine::new(connector).await?;

        Ok(Self { engine })
    }

    pub async fn handle_command<'a, E>(&'a self, input: &'a E::Input) -> CoreResult<E::Output>
    where
        E: MigrationCommand,
    {
        Ok(E::execute(input, &self.engine).await?)
    }

    pub fn connector(&self) -> &C {
        self.engine.connector()
    }
}

#[async_trait::async_trait]
pub trait GenericApi: Send + Sync + 'static {
    async fn version(&self, input: &serde_json::Value) -> CoreResult<String>;
    async fn apply_migration(&self, input: &ApplyMigrationInput) -> CoreResult<MigrationStepsResultOutput>;
    async fn apply_migrations(&self, input: &ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput>;
    async fn apply_script(&self, input: &ApplyScriptInput) -> CoreResult<ApplyScriptOutput>;
    async fn calculate_database_steps(
        &self,
        input: &CalculateDatabaseStepsInput,
    ) -> CoreResult<MigrationStepsResultOutput>;
    async fn calculate_datamodel(&self, input: &CalculateDatamodelInput) -> CoreResult<CalculateDatamodelOutput>;
    async fn create_migration(&self, input: &CreateMigrationInput) -> CoreResult<CreateMigrationOutput>;
    async fn debug_panic(&self, input: &()) -> CoreResult<()>;
    async fn diagnose_migration_history(
        &self,
        input: &DiagnoseMigrationHistoryInput,
    ) -> CoreResult<DiagnoseMigrationHistoryOutput>;
    async fn evaluate_data_loss(&self, input: &EvaluateDataLossInput) -> CoreResult<EvaluateDataLossOutput>;
    async fn infer_migration_steps(&self, input: &InferMigrationStepsInput) -> CoreResult<MigrationStepsResultOutput>;
    async fn initialize(&self, input: &InitializeInput) -> CoreResult<InitializeOutput>;
    async fn list_migrations(&self, input: &serde_json::Value) -> CoreResult<Vec<ListMigrationsOutput>>;
    async fn list_migration_directories(
        &self,
        input: &ListMigrationDirectoriesInput,
    ) -> CoreResult<ListMigrationDirectoriesOutput>;
    async fn mark_migration_applied(&self, input: &MarkMigrationAppliedInput)
        -> CoreResult<MarkMigrationAppliedOutput>;
    async fn mark_migration_rolled_back(
        &self,
        input: &MarkMigrationRolledBackInput,
    ) -> CoreResult<MarkMigrationRolledBackOutput>;
    async fn migration_progress(&self, input: &MigrationProgressInput) -> CoreResult<MigrationProgressOutput>;
    async fn plan_migration(&self, input: &PlanMigrationInput) -> CoreResult<PlanMigrationOutput>;
    async fn reset(&self, input: &()) -> CoreResult<()>;
    async fn schema_push(&self, input: &SchemaPushInput) -> CoreResult<SchemaPushOutput>;
    async fn unapply_migration(&self, input: &UnapplyMigrationInput) -> CoreResult<UnapplyMigrationOutput>;
}

#[async_trait::async_trait]
impl<C, D> GenericApi for MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    async fn version(&self, input: &serde_json::Value) -> CoreResult<String> {
        self.handle_command::<VersionCommand>(input)
            .instrument(tracing::info_span!("Version"))
            .await
    }

    async fn apply_migration(&self, input: &ApplyMigrationInput) -> CoreResult<MigrationStepsResultOutput> {
        self.handle_command::<ApplyMigrationCommand<'_>>(input)
            .instrument(tracing::info_span!(
                "ApplyMigration",
                migration_id = input.migration_id.as_str()
            ))
            .await
    }

    async fn apply_migrations(&self, input: &ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput> {
        self.handle_command::<ApplyMigrationsCommand>(input)
            .instrument(tracing::info_span!("ApplyMigrations"))
            .await
    }

    async fn apply_script(&self, input: &ApplyScriptInput) -> CoreResult<ApplyScriptOutput> {
        self.handle_command::<ApplyScriptCommand>(input)
            .instrument(tracing::info_span!("ApplyScript"))
            .await
    }

    async fn calculate_database_steps(
        &self,
        input: &CalculateDatabaseStepsInput,
    ) -> CoreResult<MigrationStepsResultOutput> {
        self.handle_command::<CalculateDatabaseStepsCommand<'_>>(input)
            .instrument(tracing::info_span!("CalculateDatabaseSteps"))
            .await
    }

    async fn calculate_datamodel(&self, input: &CalculateDatamodelInput) -> CoreResult<CalculateDatamodelOutput> {
        self.handle_command::<CalculateDatamodelCommand>(input)
            .instrument(tracing::info_span!("CalculateDatamodel"))
            .await
    }

    async fn create_migration(&self, input: &CreateMigrationInput) -> CoreResult<CreateMigrationOutput> {
        self.handle_command::<CreateMigrationCommand>(input)
            .instrument(tracing::info_span!(
                "CreateMigration",
                migration_name = input.migration_name.as_str(),
                draft = input.draft,
            ))
            .await
    }

    async fn debug_panic(&self, input: &()) -> CoreResult<()> {
        self.handle_command::<DebugPanicCommand>(input)
            .instrument(tracing::info_span!("DebugPanic"))
            .await
    }

    async fn diagnose_migration_history(
        &self,
        input: &DiagnoseMigrationHistoryInput,
    ) -> CoreResult<DiagnoseMigrationHistoryOutput> {
        self.handle_command::<DiagnoseMigrationHistoryCommand>(input)
            .instrument(tracing::info_span!("DiagnoseMigrationHistory"))
            .await
    }

    async fn evaluate_data_loss(&self, input: &EvaluateDataLossInput) -> CoreResult<EvaluateDataLossOutput> {
        self.handle_command::<EvaluateDataLoss>(input)
            .instrument(tracing::info_span!("EvaluateDataLoss"))
            .await
    }

    async fn infer_migration_steps(&self, input: &InferMigrationStepsInput) -> CoreResult<MigrationStepsResultOutput> {
        self.handle_command::<InferMigrationStepsCommand<'_>>(input)
            .instrument(tracing::info_span!(
                "InferMigrationSteps",
                migration_id = input.migration_id.as_str()
            ))
            .await
    }

    async fn initialize(&self, input: &InitializeInput) -> CoreResult<InitializeOutput> {
        self.handle_command::<InitializeCommand>(input)
            .instrument(tracing::info_span!(
                "Initialize",
                migrations_directory_path = input.migrations_directory_path.as_str()
            ))
            .await
    }

    async fn list_migrations(&self, input: &serde_json::Value) -> CoreResult<Vec<ListMigrationsOutput>> {
        self.handle_command::<ListMigrationsCommand>(input)
            .instrument(tracing::info_span!("ListMigrations"))
            .await
    }

    async fn list_migration_directories(
        &self,
        input: &ListMigrationDirectoriesInput,
    ) -> CoreResult<ListMigrationDirectoriesOutput> {
        self.handle_command::<ListMigrationDirectoriesCommand>(input)
            .instrument(tracing::info_span!("ListMigrationDirectories"))
            .await
    }

    async fn mark_migration_applied(
        &self,
        input: &MarkMigrationAppliedInput,
    ) -> CoreResult<MarkMigrationAppliedOutput> {
        self.handle_command::<MarkMigrationAppliedCommand>(input)
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
        self.handle_command::<MarkMigrationRolledBackCommand>(input)
            .instrument(tracing::info_span!(
                "MarkMigrationRolledBack",
                migration_name = input.migration_name.as_str()
            ))
            .await
    }

    async fn migration_progress(&self, input: &MigrationProgressInput) -> CoreResult<MigrationProgressOutput> {
        self.handle_command::<MigrationProgressCommand>(input)
            .instrument(tracing::info_span!(
                "MigrationProgress",
                migration_id = input.migration_id.as_str()
            ))
            .await
    }

    async fn plan_migration(&self, input: &PlanMigrationInput) -> CoreResult<PlanMigrationOutput> {
        self.handle_command::<PlanMigrationCommand>(input)
            .instrument(tracing::info_span!("PlanMigration"))
            .await
    }

    async fn reset(&self, input: &()) -> CoreResult<()> {
        self.handle_command::<ResetCommand>(input)
            .instrument(tracing::info_span!("Reset"))
            .await
    }

    async fn schema_push(&self, input: &SchemaPushInput) -> CoreResult<SchemaPushOutput> {
        self.handle_command::<SchemaPushCommand>(input)
            .instrument(tracing::info_span!("SchemaPush"))
            .await
    }

    async fn unapply_migration(&self, input: &UnapplyMigrationInput) -> CoreResult<UnapplyMigrationOutput> {
        self.handle_command::<UnapplyMigrationCommand<'_>>(input)
            .instrument(tracing::info_span!("UnapplyMigration"))
            .await
    }
}
