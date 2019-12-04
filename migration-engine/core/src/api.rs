mod error_rendering;
mod rpc;

pub use error_rendering::render_error;
pub use rpc::*;

use crate::{commands::*, migration_engine::MigrationEngine};
use migration_connector::*;
use std::sync::Arc;
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
    pub async fn new(connector: C) -> crate::Result<Self> {
        let engine = MigrationEngine::new(connector).await?;

        Ok(Self { engine })
    }

    pub async fn handle_command<'a, E>(&'a self, input: &'a E::Input) -> crate::Result<E::Output>
    where
        E: MigrationCommand,
    {
        Ok(E::execute(input, &self.engine).await?)
    }
}

// This is here only to get rid of the generic type parameters due to neon not
// liking them in the exported class.
#[async_trait::async_trait]
pub trait GenericApi: Send + Sync + 'static {
    async fn apply_migration(&self, input: &ApplyMigrationInput) -> crate::Result<MigrationStepsResultOutput>;
    async fn calculate_database_steps(
        &self,
        input: &CalculateDatabaseStepsInput,
    ) -> crate::Result<MigrationStepsResultOutput>;
    async fn calculate_datamodel(&self, input: &CalculateDatamodelInput) -> crate::Result<CalculateDatamodelOutput>;
    async fn infer_migration_steps(
        &self,
        input: &InferMigrationStepsInput,
    ) -> crate::Result<MigrationStepsResultOutput>;
    async fn list_migrations(&self, input: &serde_json::Value) -> crate::Result<Vec<ListMigrationStepsOutput>>;
    async fn migration_progress(&self, input: &MigrationProgressInput) -> crate::Result<MigrationProgressOutput>;
    async fn reset(&self, input: &serde_json::Value) -> crate::Result<serde_json::Value>;
    async fn unapply_migration(&self, input: &UnapplyMigrationInput) -> crate::Result<UnapplyMigrationOutput>;
    fn migration_persistence(&self) -> Arc<dyn MigrationPersistence>;
    fn connector_type(&self) -> &'static str;

    fn render_error(&self, error: crate::error::Error) -> user_facing_errors::Error {
        error_rendering::render_error(error)
    }

    fn render_jsonrpc_error(&self, error: crate::error::Error) -> jsonrpc_core::error::Error {
        error_rendering::render_jsonrpc_error(error)
    }

    fn render_panic(&self, panic: Box<dyn std::any::Any + Send + 'static>) -> jsonrpc_core::error::Error {
        error_rendering::render_panic(panic)
    }
}

#[async_trait::async_trait]
impl<C, D> GenericApi for MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    async fn apply_migration(&self, input: &ApplyMigrationInput) -> crate::Result<MigrationStepsResultOutput> {
        self.handle_command::<ApplyMigrationCommand>(input)
            .instrument(tracing::info_span!("ApplyMigration", migration_id = input.migration_id.as_str()))
            .await
    }

    async fn calculate_database_steps(
        &self,
        input: &CalculateDatabaseStepsInput,
    ) -> crate::Result<MigrationStepsResultOutput> {
        self.handle_command::<CalculateDatabaseStepsCommand>(input).await
    }

    async fn calculate_datamodel(&self, input: &CalculateDatamodelInput) -> crate::Result<CalculateDatamodelOutput> {
        self.handle_command::<CalculateDatamodelCommand>(input).await
    }

    async fn infer_migration_steps(
        &self,
        input: &InferMigrationStepsInput,
    ) -> crate::Result<MigrationStepsResultOutput> {
        self.handle_command::<InferMigrationStepsCommand>(input)
            .instrument(tracing::info_span!("InferMigrationSteps", migration_id = input.migration_id.as_str()))
            .await
    }

    async fn list_migrations(&self, input: &serde_json::Value) -> crate::Result<Vec<ListMigrationStepsOutput>> {
        self.handle_command::<ListMigrationStepsCommand>(input).await
    }

    async fn migration_progress(&self, input: &MigrationProgressInput) -> crate::Result<MigrationProgressOutput> {
        self.handle_command::<MigrationProgressCommand>(input).await
    }

    async fn reset(&self, input: &serde_json::Value) -> crate::Result<serde_json::Value> {
        self.handle_command::<ResetCommand>(input).await
    }

    async fn unapply_migration(&self, input: &UnapplyMigrationInput) -> crate::Result<UnapplyMigrationOutput> {
        self.handle_command::<UnapplyMigrationCommand>(input).await
    }

    fn migration_persistence(&self) -> Arc<dyn MigrationPersistence> {
        self.engine.connector().migration_persistence()
    }

    fn connector_type(&self) -> &'static str {
        self.engine.connector().connector_type()
    }
}
