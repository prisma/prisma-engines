use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use migration_connector::*;
use serde::{Deserialize, Serialize};
use tracing::debug;

pub struct UnapplyMigrationCommand<'a> {
    input: &'a UnapplyMigrationInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for UnapplyMigrationCommand<'a> {
    type Input = UnapplyMigrationInput;
    type Output = UnapplyMigrationOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + 'static,
    {
        let cmd = UnapplyMigrationCommand { input };
        debug!("{:?}", cmd.input);
        let connector = engine.connector();

        let result = match connector.migration_persistence().last().await {
            None => UnapplyMigrationOutput {
                rolled_back: "not-applicable".to_string(),
                active: None,
                errors: vec!["There is no last migration that can be rolled back.".to_string()],
            },
            Some(migration_to_rollback) => {
                let database_migration =
                    connector.deserialize_database_migration(migration_to_rollback.database_migration.clone());

                connector
                    .migration_applier()
                    .unapply(&migration_to_rollback, &database_migration)
                    .await?;

                let new_active_migration = connector.migration_persistence().last().await.map(|m| m.name);

                UnapplyMigrationOutput {
                    rolled_back: migration_to_rollback.name,
                    active: new_active_migration,
                    errors: Vec::new(),
                }
            }
        };

        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnapplyMigrationInput {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnapplyMigrationOutput {
    pub rolled_back: String,
    pub active: Option<String>,
    pub errors: Vec<String>,
}
