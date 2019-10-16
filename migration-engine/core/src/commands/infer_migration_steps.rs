use super::MigrationStepsResultOutput;
use crate::commands::command::*;
use crate::migration_engine::MigrationEngine;
use crate::*;
use migration_connector::*;

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
        let current_datamodel = migration_persistence.current_datamodel();
        let assumed_datamodel = engine
            .datamodel_calculator()
            .infer(&current_datamodel, &self.input.assume_to_be_applied);

        let next_datamodel = parse_datamodel(&self.input.datamodel)?;

        let model_migration_steps = engine
            .datamodel_migration_steps_inferrer()
            .infer(&assumed_datamodel, &next_datamodel);

        let database_migration = connector.database_migration_inferrer().infer(
            &assumed_datamodel,
            &next_datamodel,
            &model_migration_steps,
        )?;

        let DestructiveChangeDiagnostics { warnings, errors: _ } =
            connector.destructive_changes_checker().check(&database_migration)?;

        let database_steps = connector
            .database_migration_step_applier()
            .render_steps_pretty(&database_migration)?;

        let (returned_datamodel_steps, returned_database_migration) = if self.input.is_watch_migration() {
            (model_migration_steps, database_steps)
        } else {
            let watch_migrations = migration_persistence.load_current_watch_migrations();

            let mut returned_datamodel_steps = Vec::new();
            let mut returned_database_steps = Vec::new();

            for migration in watch_migrations {
                let database_migration: D = serde_json::from_value(migration.database_migration)
                    .expect("Database migration can be deserialized.");
                let database_migration_steps: Vec<serde_json::Value> = connector
                    .database_migration_step_applier()
                    .render_steps_pretty(&database_migration)?;

                returned_datamodel_steps.extend(migration.datamodel_steps);
                returned_database_steps.extend(database_migration_steps);
            }

            returned_datamodel_steps.extend(model_migration_steps);
            returned_database_steps.extend(database_steps);

            (returned_datamodel_steps, returned_database_steps)
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
