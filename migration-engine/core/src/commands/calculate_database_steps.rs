//! The CalculateDatabaseSteps RPC method.
//!
//! Its purpose is to infer the database steps for a given migration without reference to a target
//! prisma schema/datamodel, based on the datamodel migration steps and previous already applied
//! migrations.

use super::{AppliedMigration, MigrationStepsResultOutput};
use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use datamodel::ast::SchemaAst;
use migration_connector::*;
use serde::Deserialize;
use tracing::debug;

pub struct CalculateDatabaseStepsCommand<'a> {
    input: &'a CalculateDatabaseStepsInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for CalculateDatabaseStepsCommand<'a> {
    type Input = CalculateDatabaseStepsInput;
    type Output = MigrationStepsResultOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let cmd = CalculateDatabaseStepsCommand { input };
        debug!(command_input = ?cmd.input);

        let connector = engine.connector();
        let migration_persistence = connector.migration_persistence();

        let assume_to_be_applied = cmd.assume_to_be_applied();
        cmd.validate_assumed_migrations_are_not_applied(migration_persistence.as_ref())
            .await?;

        let assumed_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&SchemaAst::empty(), &assume_to_be_applied)?;
        let assumed_datamodel = datamodel::lift_ast(&assumed_datamodel_ast)?;

        let next_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&assumed_datamodel_ast, &cmd.input.steps_to_apply)?;
        let next_datamodel = datamodel::lift_ast(&next_datamodel_ast)?;

        let database_migration = connector
            .database_migration_inferrer()
            .infer(&assumed_datamodel, &next_datamodel, &cmd.input.steps_to_apply)
            .await?;

        let DestructiveChangeDiagnostics {
            warnings,
            errors: _,
            unexecutable_migrations,
        } = connector
            .destructive_changes_checker()
            .check(&database_migration)
            .await?;

        let database_steps_json = connector
            .database_migration_step_applier()
            .render_steps_pretty(&database_migration)?;

        Ok(MigrationStepsResultOutput {
            datamodel: datamodel::render_schema_ast_to_string(&next_datamodel_ast).unwrap(),
            datamodel_steps: cmd.input.steps_to_apply.clone(),
            database_steps: serde_json::Value::Array(database_steps_json),
            errors: Vec::new(),
            warnings,
            general_errors: Vec::new(),
            unexecutable_migrations,
        })
    }
}

impl CalculateDatabaseStepsCommand<'_> {
    fn assume_to_be_applied(&self) -> Vec<MigrationStep> {
        self.input
            .assume_to_be_applied
            .clone()
            .or_else(|| {
                self.input.assume_applied_migrations.as_ref().map(|migrations| {
                    migrations
                        .into_iter()
                        .flat_map(|migration| migration.datamodel_steps.clone().into_iter())
                        .collect()
                })
            })
            .unwrap_or_else(Vec::new)
    }

    async fn validate_assumed_migrations_are_not_applied(
        &self,
        migration_persistence: &dyn MigrationPersistence,
    ) -> CommandResult<()> {
        if let Some(migrations) = self.input.assume_applied_migrations.as_ref() {
            for migration in migrations {
                if migration_persistence
                    .migration_is_already_applied(&migration.migration_id)
                    .await?
                {
                    return Err(CommandError::ConnectorError(ConnectorError {
                        user_facing_error: None,
                        kind: ErrorKind::Generic(anyhow::anyhow!(
                            "Input is invalid. Migration {} is already applied.",
                            migration.migration_id
                        )),
                    }));
                }
            }
        }

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDatabaseStepsInput {
    pub steps_to_apply: Vec<MigrationStep>,
    /// Migration steps from migrations that have been inferred but not applied yet.
    ///
    /// These steps must be provided and correct for migration inferrence to work.
    pub assume_to_be_applied: Option<Vec<MigrationStep>>,
    pub assume_applied_migrations: Option<Vec<AppliedMigration>>,
}
