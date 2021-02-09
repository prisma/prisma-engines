mod error_rendering;
mod rpc;

pub use rpc::RpcApi;

use crate::{commands::*, CoreResult};
use migration_connector::MigrationConnector;
use tracing_futures::Instrument;

pub struct MigrationApi<C>
where
    C: MigrationConnector,
{
    connector: C,
}

impl<C: MigrationConnector> MigrationApi<C> {
    pub fn new(connector: C) -> Self {
        MigrationApi { connector }
    }

    pub async fn handle_command<'a, E>(&'a self, input: &'a E::Input) -> CoreResult<E::Output>
    where
        E: MigrationCommand,
    {
        Ok(E::execute(input, self.connector()).await?)
    }

    pub fn connector(&self) -> &C {
        &self.connector
    }
}

#[async_trait::async_trait]
pub trait GenericApi: Send + Sync + 'static {
    async fn version(&self, input: &serde_json::Value) -> CoreResult<String>;
    async fn apply_migrations(&self, input: &ApplyMigrationsInput) -> CoreResult<ApplyMigrationsOutput>;
    async fn apply_script(&self, input: &ApplyScriptInput) -> CoreResult<ApplyScriptOutput>;
    async fn create_migration(&self, input: &CreateMigrationInput) -> CoreResult<CreateMigrationOutput>;
    async fn debug_panic(&self, input: &()) -> CoreResult<()>;
    async fn dev_diagnostic(&self, input: &DevDiagnosticInput) -> CoreResult<DevDiagnosticOutput>;
    async fn diagnose_migration_history(
        &self,
        input: &DiagnoseMigrationHistoryInput,
    ) -> CoreResult<DiagnoseMigrationHistoryOutput>;
    async fn evaluate_data_loss(&self, input: &EvaluateDataLossInput) -> CoreResult<EvaluateDataLossOutput>;
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
    async fn plan_migration(&self, input: &PlanMigrationInput) -> CoreResult<PlanMigrationOutput>;
    async fn reset(&self, input: &()) -> CoreResult<()>;
    async fn schema_push(&self, input: &SchemaPushInput) -> CoreResult<SchemaPushOutput>;
}

#[async_trait::async_trait]
impl<C: MigrationConnector> GenericApi for MigrationApi<C> {
    async fn version(&self, input: &serde_json::Value) -> CoreResult<String> {
        self.handle_command::<VersionCommand>(input)
            .instrument(tracing::info_span!("Version"))
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

    async fn dev_diagnostic(&self, input: &DevDiagnosticInput) -> CoreResult<DevDiagnosticOutput> {
        self.handle_command::<DevDiagnosticCommand>(input)
            .instrument(tracing::info_span!("DevDiagnostic"))
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
}
