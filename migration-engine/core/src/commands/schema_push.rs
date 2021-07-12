use crate::{parse_schema, CoreResult};
use migration_connector::{ConnectorError, DiffTarget, MigrationConnector};
use serde::{Deserialize, Serialize};

/// Command to bring the local database in sync with the prisma schema, without
/// interacting with the migrations directory nor the migrations table.

pub(crate) async fn schema_push(
    input: &SchemaPushInput,
    connector: &dyn MigrationConnector,
) -> CoreResult<SchemaPushOutput> {
    let schema = parse_schema(&input.schema)?;
    let applier = connector.database_migration_step_applier();
    let checker = connector.destructive_change_checker();

    if let Some(err) = connector.check_database_version_compatibility(&schema.1) {
        return Err(ConnectorError::user_facing(err));
    };

    let from = if input.assume_empty {
        DiffTarget::Empty
    } else {
        DiffTarget::Database
    };

    let database_migration = connector
        .diff(from, DiffTarget::Datamodel((&schema.0, &schema.1)))
        .await?;

    let checks = checker.check(&database_migration).await?;

    let executed_steps = match (checks.unexecutable_migrations.len(), checks.warnings.len(), input.force) {
        (unexecutable, _, _) if unexecutable > 0 => {
            tracing::warn!(unexecutable = ?checks.unexecutable_migrations, "Aborting migration because at least one unexecutable step was detected.");

            0
        }
        (0, 0, _) | (0, _, true) => applier.apply_migration(&database_migration).await?,
        _ => {
            tracing::info!(
                "The migration was not applied because it triggered warnings and the force flag was not passed."
            );

            0
        }
    };

    Ok(SchemaPushOutput {
        executed_steps,
        warnings: checks.warnings.into_iter().map(|warning| warning.description).collect(),
        unexecutable: checks
            .unexecutable_migrations
            .into_iter()
            .map(|unexecutable| unexecutable.description)
            .collect(),
    })
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
