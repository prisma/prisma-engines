use super::{introspect_database, TestSetup};
use migration_connector::*;
use migration_core::{api::GenericApi, commands::*};
use sql_migration_connector::{PrettySqlMigrationStep, SqlMigrationStep};
use sql_schema_describer::*;

#[derive(Debug)]
pub struct InferAndApplyOutput {
    pub sql_schema: SqlSchema,
    pub migration_output: MigrationStepsResultOutput,
}

impl InferAndApplyOutput {
    pub fn sql_migration(&self) -> Vec<SqlMigrationStep> {
        let steps: Vec<PrettySqlMigrationStep> =
            serde_json::from_value(self.migration_output.database_steps.clone()).unwrap();
        steps.into_iter().map(|pretty_step| pretty_step.step).collect()
    }
}

pub fn infer_and_apply(test_setup: &TestSetup, api: &dyn GenericApi, datamodel: &str) -> InferAndApplyOutput {
    infer_and_apply_with_migration_id(test_setup, api, &datamodel, "the-migration-id")
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

    let steps = run_infer_command(api, input).0.datamodel_steps;

    apply_migration(test_setup, api, steps, migration_id)
}

#[derive(Debug)]
pub struct InferOutput(pub MigrationStepsResultOutput);

impl InferOutput {
    pub fn sql_migration(&self) -> Vec<SqlMigrationStep> {
        let steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(self.0.database_steps.clone()).unwrap();
        steps.into_iter().map(|pretty_step| pretty_step.step).collect()
    }
}

pub fn run_infer_command(api: &dyn GenericApi, input: InferMigrationStepsInput) -> InferOutput {
    let output = api.infer_migration_steps(&input).expect("InferMigration failed");

    assert!(
        output.general_errors.is_empty(),
        format!("InferMigration returned unexpected errors: {:?}", output.general_errors)
    );

    InferOutput(output)
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

    let migration_output = dbg!(api.apply_migration(&input)).expect("ApplyMigration failed");

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
