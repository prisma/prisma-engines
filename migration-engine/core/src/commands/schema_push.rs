use super::MigrationCommand;
use crate::parse_datamodel;
use migration_connector::{DatabaseMigrationMarker, MigrationConnector};
use serde::{Deserialize, Serialize};

pub struct SchemaPushCommand<'a> {
    pub input: &'a SchemaPushInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for SchemaPushCommand<'a> {
    type Input = SchemaPushInput;
    type Output = SchemaPushOutput;

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

        let database_migration = inferrer.infer(&schema, &schema, &[]).await?;

        let checks = checker.check(&database_migration).await?;

        let mut step = 0u32;

        match (checks.unexecutable_migrations.len(), checks.warnings.len(), input.force) {
            (unexecutable, _, _) if unexecutable > 0 => {
                tracing::warn!(unexecutable = ?checks.unexecutable_migrations, "Aborting migration because at least one unexecutable step was detected.")
            }
            (0, 0, _) | (0, _, true) => {
                while applier.apply_step(&database_migration, step as usize).await? {
                    step += 1
                }
            }
            _ => tracing::info!(
                "The migration was not applied because it triggered warnings and the force flag was not passed."
            ),
        }

        Ok(SchemaPushOutput {
            executed_steps: step,
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
pub struct SchemaPushInput {
    /// The prisma schema.
    pub schema: String,
    /// Push the schema ignoring destructive change warnings.
    pub force: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaPushOutput {
    pub executed_steps: u32,
    pub warnings: Vec<String>,
    pub unexecutable: Vec<String>,
}

impl SchemaPushOutput {
    pub fn had_no_changes_to_push(&self) -> bool {
        self.warnings.is_empty() && self.unexecutable.is_empty() && self.executed_steps == 0
    }
}
