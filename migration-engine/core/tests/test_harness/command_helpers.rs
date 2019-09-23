use super::{introspect_database, TestSetup};
use migration_connector::*;
use migration_core::{api::GenericApi, commands::*};
use sql_migration_connector::SqlMigrationStep;
use sql_schema_describer::*;

#[derive(Debug)]
pub struct InferAndApplyOutput {
    pub sql_schema: SqlSchema,
    pub migration_output: MigrationStepsResultOutput,
}

impl InferAndApplyOutput {
    pub fn sql_migration(&self) -> Vec<SqlMigrationStep> {
        let mut steps = self.migration_output.database_steps.clone();
        steps.as_array_mut().unwrap().iter_mut().for_each(|value| {
            value.as_object_mut().unwrap().remove("raw");
        });
        serde_json::from_value(steps).unwrap()
    }
}

pub fn infer_and_apply(api: &dyn GenericApi, datamodel: &str) -> InferAndApplyOutput {
    infer_and_apply_with_migration_id(api, &datamodel, "the-migration-id")
}

pub fn infer_and_apply_with_migration_id(
    test_setup: &TestSetup,
    api: &dyn GenericApi,
    datamodel: &str,
    migration_id: &str,
) -> InferAndApplyOutput {
    let input = InferMigrationStepsInput {
        migration_id: migration_id.to_string(),
        datamodel: datamodel.to_string(),
        assume_to_be_applied: Vec::new(),
    };

    let steps = run_infer_command(api, input);

    apply_migration(test_setup, api, steps, migration_id)
}

pub fn run_infer_command(api: &dyn GenericApi, input: InferMigrationStepsInput) -> Vec<MigrationStep> {
    let output = api.infer_migration_steps(&input).expect("InferMigration failed");

    assert!(
        output.general_errors.is_empty(),
        format!("InferMigration returned unexpected errors: {:?}", output.general_errors)
    );

    output.datamodel_steps
}

pub fn apply_migration(
    test_setup: &TestSetup,
    api: &dyn GenericApi,
    steps: Vec<MigrationStep>,
    migration_id: &str,
) -> InferAndApplyOutput {
    let input = ApplyMigrationInput {
        migration_id: migration_id.to_string(),
        steps: steps,
        force: None,
    };

    let migration_output = api.apply_migration(&input).expect("ApplyMigration failed");

    assert!(
        migration_output.general_errors.is_empty(),
        format!(
            "ApplyMigration returned unexpected errors: {:?}",
            migration_output.general_errors
        )
    );

    InferAndApplyOutput {
        sql_schema: introspect_database(test_setup, api),
        migration_output,
    }
}

pub fn unapply_migration(test_setup: &TestSetup, api: &dyn GenericApi) -> SqlSchema {
    let input = UnapplyMigrationInput {};
    let _ = api.unapply_migration(&input);

    introspect_database(test_setup, api)
}
