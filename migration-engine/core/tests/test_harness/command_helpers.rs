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

#[derive(Debug)]
pub struct InferOutput(pub MigrationStepsResultOutput);

impl InferOutput {
    pub fn sql_migration(&self) -> Vec<SqlMigrationStep> {
        let steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(self.0.database_steps.clone()).unwrap();
        steps.into_iter().map(|pretty_step| pretty_step.step).collect()
    }
}

pub(super) async fn run_infer_command(api: &dyn GenericApi, input: InferMigrationStepsInput) -> InferOutput {
    let output = api.infer_migration_steps(&input).await.expect("InferMigration failed");

    assert!(
        output.general_errors.is_empty(),
        format!("InferMigration returned unexpected errors: {:?}", output.general_errors)
    );

    InferOutput(output)
}
