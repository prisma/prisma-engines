//! The InferMigrationSteps RPC method.

use super::MigrationStepsResultOutput;
use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use crate::*;
use datamodel::ast::{parser::parse, SchemaAst};
use migration_connector::*;
use serde::Deserialize;
use tracing::debug;

pub struct InferMigrationStepsCommand<'a> {
    input: &'a InferMigrationStepsInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for InferMigrationStepsCommand<'a> {
    type Input = InferMigrationStepsInput;
    type Output = MigrationStepsResultOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Sync + Send + 'static,
    {
        let cmd = InferMigrationStepsCommand { input };
        debug!(?cmd.input);

        let connector = engine.connector();
        let migration_persistence = connector.migration_persistence();
        let database_migration_inferrer = connector.database_migration_inferrer();

        let assume_to_be_applied = cmd.assume_to_be_applied();

        cmd.validate_assumed_migrations_are_not_applied(migration_persistence.as_ref())
            .await?;

        let current_datamodel_ast = migration_persistence.current_datamodel_ast().await?;
        let assumed_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&current_datamodel_ast, assume_to_be_applied.as_slice())?;
        let assumed_datamodel = datamodel::lift_ast(&assumed_datamodel_ast)?;

        let next_datamodel = parse_datamodel(&cmd.input.datamodel)?;
        let next_datamodel_ast = parse(&cmd.input.datamodel)?;

        let model_migration_steps = engine
            .datamodel_migration_steps_inferrer()
            .infer(&assumed_datamodel_ast, &next_datamodel_ast);

        let database_migration = database_migration_inferrer
            .infer(&assumed_datamodel, &next_datamodel, &model_migration_steps)
            .await?;

        let DestructiveChangeDiagnostics { warnings, errors: _ } = connector
            .destructive_changes_checker()
            .check(&database_migration)
            .await?;

        let (returned_datamodel_steps, returned_database_migration) = {
            let last_applied_migration = migration_persistence.last_applied_migration().await?;
            let last_datamodel_ast = last_applied_migration
                .as_ref()
                .map(|m| m.datamodel_ast())
                .unwrap_or_else(SchemaAst::empty);
            let last_datamodel = last_applied_migration
                .map(|m| m.parse_datamodel())
                .unwrap_or_else(Datamodel::empty);
            let datamodel_steps = engine
                .datamodel_migration_steps_inferrer()
                .infer(&last_datamodel_ast, &next_datamodel_ast);

            let full_database_migration = database_migration_inferrer
                .infer_from_datamodels(&last_datamodel, &next_datamodel, &datamodel_steps)
                .await?;
            let database_steps = connector
                .database_migration_step_applier()
                .render_steps_pretty(&full_database_migration)?;

            (datamodel_steps, database_steps)
        };

        debug!(?returned_datamodel_steps);

        Ok(MigrationStepsResultOutput {
            datamodel: datamodel::render_datamodel_to_string(&next_datamodel).unwrap(),
            datamodel_steps: returned_datamodel_steps,
            database_steps: serde_json::Value::Array(returned_database_migration),
            errors: vec![],
            warnings,
            general_errors: vec![],
        })
    }
}

impl InferMigrationStepsCommand<'_> {
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferMigrationStepsInput {
    pub migration_id: String,
    #[serde(alias = "dataModel")]
    pub datamodel: String,
    /// Migration steps from migrations that have been inferred but not applied yet.
    ///
    /// These steps must be provided and correct for migration inferrence to work.
    pub assume_to_be_applied: Option<Vec<MigrationStep>>,
    pub assume_applied_migrations: Option<Vec<AppliedMigration>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedMigration {
    pub migration_id: String,
    pub datamodel_steps: Vec<MigrationStep>,
}
