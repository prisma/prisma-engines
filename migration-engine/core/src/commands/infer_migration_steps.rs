use super::MigrationStepsResultOutput;
use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use crate::*;
use log::*;
use migration_connector::*;
use serde::Deserialize;

pub struct InferMigrationStepsCommand<'a> {
    input: &'a InferMigrationStepsInput,
}

impl<'a> MigrationCommand<'a> for InferMigrationStepsCommand<'a> {
    type Input = InferMigrationStepsInput;
    type Output = MigrationStepsResultOutput;

    fn new(input: &'a Self::Input) -> Box<Self> {
        Box::new(InferMigrationStepsCommand { input })
    }

    fn execute<C, D>(&self, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Sync + Send + 'static,
    {
        debug!("{:?}", self.input);

        let connector = engine.connector();
        let migration_persistence = connector.migration_persistence();
        let database_migration_inferrer = connector.database_migration_inferrer();

        let current_datamodel = migration_persistence.current_datamodel();
        let assumed_datamodel = engine
            .datamodel_calculator()
            .infer(&current_datamodel, &self.input.assume_to_be_applied);

        let next_datamodel = parse_datamodel(&self.input.datamodel)?;

        let model_migration_steps = engine
            .datamodel_migration_steps_inferrer()
            .infer(&assumed_datamodel, &next_datamodel);

        let database_migration =
            database_migration_inferrer.infer(&assumed_datamodel, &next_datamodel, &model_migration_steps)?;

        let DestructiveChangeDiagnostics { warnings, errors: _ } =
            connector.destructive_changes_checker().check(&database_migration)?;

        let (returned_datamodel_steps, returned_database_migration) = if self.input.is_watch_migration() {
            let database_steps = connector
                .database_migration_step_applier()
                .render_steps_pretty(&database_migration)?;

            (model_migration_steps, database_steps)
        } else {
            let last_non_watch_applied_migration = migration_persistence.last_non_watch_applied_migration();
            let last_non_watch_datamodel = last_non_watch_applied_migration
                .map(|m| m.datamodel)
                .unwrap_or_else(Datamodel::empty);
            let datamodel_steps = engine
                .datamodel_migration_steps_inferrer()
                .infer(&last_non_watch_datamodel, &next_datamodel);

            // The database migration since the last non-watch migration, so we can render all the steps applied
            // in watch mode to the migrations folder.
            let full_database_migration = database_migration_inferrer.infer_from_datamodels(
                &last_non_watch_datamodel,
                &next_datamodel,
                &datamodel_steps,
            )?;
            let database_steps = connector
                .database_migration_step_applier()
                .render_steps_pretty(&full_database_migration)?;

            (datamodel_steps, database_steps)
        };

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferMigrationStepsInput {
    pub migration_id: String,
    #[serde(alias = "dataModel")]
    pub datamodel: String,
    pub assume_to_be_applied: Vec<MigrationStep>,
}

impl IsWatchMigration for InferMigrationStepsInput {
    fn is_watch_migration(&self) -> bool {
        self.migration_id.starts_with("watch")
    }
}
