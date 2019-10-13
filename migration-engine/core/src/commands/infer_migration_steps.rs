use super::MigrationStepsResultOutput;
use crate::commands::command::*;
use crate::migration::datamodel_differ;
use crate::migration_engine::MigrationEngine;
use crate::*;
use datamodel::ast::parser::parse;
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
        let current_datamodel_ast = migration_persistence.current_datamodel_ast();
        let assumed_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&current_datamodel_ast, self.input.assume_to_be_applied.as_slice());
        let assumed_datamodel = datamodel::lift_ast(&assumed_datamodel_ast)?;

        let next_datamodel = parse_datamodel(&self.input.datamodel)?;
        let next_datamodel_ast = parse(&self.input.datamodel)?;

        let model_migration_steps = engine
            .datamodel_migration_steps_inferrer()
            .infer(&assumed_datamodel_ast, &next_datamodel_ast);

        let database_migration = connector.database_migration_inferrer().infer(
            &assumed_datamodel,
            &next_datamodel,
            &model_migration_steps,
        )?;

        let DestructiveChangeDiagnostics { warnings, errors: _ } =
            connector.destructive_changes_checker().check(&database_migration)?;

        let database_steps_json = connector
            .database_migration_step_applier()
            .render_steps_pretty(&database_migration)?;

        let returned_datamodel_steps = if self.input.is_watch_migration() {
            model_migration_steps
        } else {
            let mut steps = migration_persistence.load_all_datamodel_steps_from_all_current_watch_migrations();
            steps.append(&mut model_migration_steps.clone());
            steps
        };

        Ok(MigrationStepsResultOutput {
            datamodel: datamodel::render_datamodel_to_string(&next_datamodel).unwrap(),
            datamodel_steps: returned_datamodel_steps,
            database_steps: database_steps_json,
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
