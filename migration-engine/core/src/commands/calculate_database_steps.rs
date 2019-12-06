//! The CalculateDatabaseSteps RPC method.

use super::MigrationStepsResultOutput;
use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use datamodel::ast::SchemaAst;
use tracing::debug;
use migration_connector::*;
use serde::Deserialize;

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

        let assumed_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&SchemaAst::empty(), &cmd.input.assume_to_be_applied)?;
        let assumed_datamodel = datamodel::lift_ast(&assumed_datamodel_ast)?;

        let next_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&assumed_datamodel_ast, &cmd.input.steps_to_apply)?;
        let next_datamodel = datamodel::lift_ast(&next_datamodel_ast)?;

        let database_migration = connector
            .database_migration_inferrer()
            .infer(&assumed_datamodel, &next_datamodel, &cmd.input.steps_to_apply)
            .await?;

        let DestructiveChangeDiagnostics { warnings, errors: _ } = connector
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
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDatabaseStepsInput {
    pub assume_to_be_applied: Vec<MigrationStep>,
    pub steps_to_apply: Vec<MigrationStep>,
}
