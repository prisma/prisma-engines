use crate::commands::*;
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

pub trait MigrationStepsResultOutputExt {
    fn sql_migration(&self) -> Vec<SqlMigrationStep>;
}

impl MigrationStepsResultOutputExt for MigrationStepsResultOutput {
    fn sql_migration(&self) -> Vec<SqlMigrationStep> {
        let steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(self.database_steps.clone()).unwrap();
        steps.into_iter().map(|pretty_step| pretty_step.step).collect()
    }
}
