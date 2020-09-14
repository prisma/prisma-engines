use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use migration_connector::*;
use serde::Deserialize;

/// Returns the version of the used db if available.
pub struct VersionCommand<'a> {
    input: &'a VersionInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for VersionCommand<'a> {
    type Input = VersionInput;
    type Output = String;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let cmd = VersionCommand { input };
        tracing::debug!("{:?}", cmd.input);

        let connector = engine.connector();
        Ok(connector.version())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// The input to the `Version` command.
pub struct VersionInput {}
