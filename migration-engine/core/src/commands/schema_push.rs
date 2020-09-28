use super::MigrationCommand;
use crate::parse_datamodel;
use migration_connector::{DatabaseMigrationMarker, MigrationConnector};
use serde::{Deserialize, Serialize};

/// Command to bring the local database in sync with the prisma schema, without
/// interacting with the migrations directory nor the migrations table.
pub struct SchemaPushCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for SchemaPushCommand {
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
        let checker = connector.destructive_change_checker();

        let database_migration = if input.assume_empty {
            inferrer.infer_from_empty(&schema)?
        } else {
            inferrer.infer(&schema, &schema, &[]).await?
        };

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

/// Input to the `schemaPush` command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaPushInput {
    /// The prisma schema.
    pub schema: String,
    /// Push the schema ignoring destructive change warnings.
    pub force: bool,
    /// Expect the schema to be empty, skipping describing the existing schema.
    #[serde(default)]
    pub assume_empty: bool,
}

/// Output of the `schemaPush` command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaPushOutput {
    /// How many migration steps were executed.
    pub executed_steps: u32,
    /// Destructive change warnings.
    pub warnings: Vec<String>,
    /// Steps that cannot be executed in the current state of the database.
    pub unexecutable: Vec<String>,
}

impl SchemaPushOutput {
    /// Returns whether the local database schema is in sync with the prisma schema.
    pub fn had_no_changes_to_push(&self) -> bool {
        self.warnings.is_empty() && self.unexecutable.is_empty() && self.executed_steps == 0
    }
}
