use super::{CommandError, MigrationCommand};
use crate::parse_datamodel;
use datamodel::Datamodel;
use migration_connector::{DatabaseMigrationMarker, MigrationConnector};
use serde::{Deserialize, Serialize};

pub struct PushSchemaCommand<'a> {
    input: &'a PushSchemaInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for PushSchemaCommand<'a> {
    type Input = PushSchemaInput;
    type Output = PushSchemaOutput;

    async fn execute<C, D>(
        input: &Self::Input,
        engine: &crate::migration_engine::MigrationEngine<C, D>,
    ) -> super::CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let schema = parse_datamodel(&input.schema)?;
        let inferrer = connector.database_migration_inferrer();
        let applier = connector.database_migration_step_applier();
        let checker = connector.destructive_changes_checker();

        let sql_migration = inferrer.infer(&schema, &schema, &[]).await?;

        let checks = checker.check(&sql_migration).await?;

        if !checks.unexecutable_migrations.is_empty() {}

        match (checks.warnings.len(), input.force) {
            (0, _) | (_, true) => {
                let mut step = 0;

                while applier.apply_step(&sql_migration, step).await? {
                    step += 1
                }
            }
            _ => (),
        }

        Ok(PushSchemaOutput {
            warnings: checks.warnings.into_iter().map(|warning| warning.description).collect(),
            unexecutable: checks
                .unexecutable_migrations
                .into_iter()
                .map(|unexecutable| unexecutable.description)
                .collect(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushSchemaInput {
    /// The prisma schema.
    pub schema: String,
    /// Push the schema ignoring destructive change warnings.
    pub force: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushSchemaOutput {
    warnings: Vec<String>,
    unexecutable: Vec<String>,
}
