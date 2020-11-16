use super::MigrationCommand;
use crate::{migration_engine::MigrationEngine, CoreResult};
use serde::Deserialize;
use std::collections::HashMap;

/// The input to the `applyScript` command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyScriptInput {
    /// The script as a string. This will become more interesting when we
    /// implement migration DSLs.
    pub script: String,
}

/// The output of the `applyScript` command.
pub type ApplyScriptOutput = HashMap<(), ()>;

/// Apply a script to the database without recording anything in the migrations
/// table. This is currently used for correcting drift.
pub struct ApplyScriptCommand;

#[async_trait::async_trait]
impl MigrationCommand for ApplyScriptCommand {
    type Input = ApplyScriptInput;

    type Output = ApplyScriptOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let applier = engine.connector().database_migration_step_applier();

        applier.apply_script(&input.script).await?;

        Ok(Default::default())
    }
}
