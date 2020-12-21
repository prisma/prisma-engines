use crate::migration_engine::MigrationEngine;
use crate::{commands::command::*, CoreResult};
use migration_connector::*;

/// Returns the version of the used db if available.
pub struct VersionCommand;

#[async_trait::async_trait]
impl MigrationCommand for VersionCommand {
    type Input = serde_json::Value;
    type Output = String;

    async fn execute<C: MigrationConnector>(
        _input: &Self::Input,
        engine: &MigrationEngine<C>,
    ) -> CoreResult<Self::Output> {
        let connector = engine.connector();
        Ok(connector.version().await?)
    }
}
