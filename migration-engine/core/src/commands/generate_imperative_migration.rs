use super::MigrationCommand;
use crate::parse_datamodel;
use migration_connector::ImperativeMigration;
use migration_connector::{DatabaseMigrationMarker, MigrationConnector};
use serde::{Deserialize, Serialize};

pub struct GenerateImperativeMigrationCommand<'a> {
    input: &'a GenerateImperativeMigrationInput,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateImperativeMigrationInput {
    target_schema: String,
    migrations: Vec<ImperativeMigration>,
    migration_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateImperativeMigrationOutput {
    warnings: Vec<String>,
    unexecutable: Vec<String>,
    migration: ImperativeMigration,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for GenerateImperativeMigrationCommand<'a> {
    type Input = GenerateImperativeMigrationInput;
    type Output = GenerateImperativeMigrationOutput;

    async fn execute<C, D>(
        input: &Self::Input,
        engine: &crate::migration_engine::MigrationEngine<C, D>,
    ) -> super::CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let checker = connector.destructive_changes_checker();

        let schema_string = &input.target_schema;
        let schema = parse_datamodel(schema_string)?;

        let (imperative_migration, database_migration) = connector
            .generate_imperative_migration(&input.migrations, &schema, schema_string, &input.migration_name)
            .await?;

        let check_results = checker.pure_check(&database_migration)?;

        let response = GenerateImperativeMigrationOutput {
            warnings: check_results
                .warnings
                .into_iter()
                .map(|warning| warning.description)
                .collect(),
            unexecutable: check_results
                .unexecutable_migrations
                .into_iter()
                .map(|unexecutable| unexecutable.description)
                .collect(),
            migration: imperative_migration,
        };

        Ok(response)
    }
}
