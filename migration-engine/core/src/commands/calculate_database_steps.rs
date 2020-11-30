//! The CalculateDatabaseSteps RPC method.
//!
//! Its purpose is to infer the database steps for a given migration without reference to a target
//! prisma schema/datamodel, based on the datamodel migration steps and previous already applied
//! migrations.

use super::MigrationStepsResultOutput;
use crate::{commands::command::MigrationCommand, migration_engine::MigrationEngine, CoreError, CoreResult};
use datamodel::ast::SchemaAst;
use migration_connector::{DatabaseMigrationMarker, DestructiveChangeDiagnostics, MigrationConnector, MigrationStep};
use serde::Deserialize;

pub struct CalculateDatabaseStepsCommand<'a> {
    input: &'a CalculateDatabaseStepsInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for CalculateDatabaseStepsCommand<'a> {
    type Input = CalculateDatabaseStepsInput;
    type Output = MigrationStepsResultOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let cmd = CalculateDatabaseStepsCommand { input };
        tracing::debug!(command_input = ?cmd.input);

        let connector = engine.connector();

        let steps_to_apply = &cmd.input.steps_to_apply;
        let assume_to_be_applied = cmd.applicable_steps();

        let assumed_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&SchemaAst::empty(), &assume_to_be_applied)?;
        let assumed_datamodel =
            datamodel::lift_ast_to_datamodel(&assumed_datamodel_ast).map_err(CoreError::ProducedBadDatamodel)?;

        let next_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&assumed_datamodel_ast, &steps_to_apply)?;
        let next_datamodel =
            datamodel::lift_ast_to_datamodel(&next_datamodel_ast).map_err(CoreError::ProducedBadDatamodel)?;

        let database_migration = connector
            .database_migration_inferrer()
            .infer(&assumed_datamodel.subject, &next_datamodel.subject, &steps_to_apply)
            .await?;

        let DestructiveChangeDiagnostics {
            warnings,
            unexecutable_migrations,
        } = connector
            .destructive_change_checker()
            .check(&database_migration)
            .await?;

        let database_steps_json = connector
            .database_migration_step_applier()
            .render_steps_pretty(&database_migration)?;

        Ok(MigrationStepsResultOutput {
            datamodel: datamodel::render_schema_ast_to_string(&next_datamodel_ast),
            datamodel_steps: steps_to_apply.to_vec(),
            database_steps: database_steps_json,
            errors: [],
            warnings,
            general_errors: [],
            unexecutable_migrations,
        })
    }
}

impl CalculateDatabaseStepsCommand<'_> {
    /// Returns assume_to_be_applied from the input, with the exception of the steps from
    /// steps_to_apply that may have been sent by mistake.
    fn applicable_steps(&self) -> &[MigrationStep] {
        match self.input.assume_to_be_applied.as_ref() {
            Some(all_steps) => {
                let steps_to_apply = &self.input.steps_to_apply;

                if steps_to_apply.len() >= all_steps.len() {
                    return all_steps;
                }

                let start_idx = all_steps.len() - (steps_to_apply.len());
                let sliced = &all_steps[start_idx..];

                if sliced == steps_to_apply.as_slice() {
                    return &all_steps[..start_idx];
                }

                all_steps
            }
            None => &[],
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDatabaseStepsInput {
    pub steps_to_apply: Vec<MigrationStep>,
    pub assume_to_be_applied: Option<Vec<MigrationStep>>,
}
