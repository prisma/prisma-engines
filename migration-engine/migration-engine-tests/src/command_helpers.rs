use migration_core::commands::*;
use sql_schema_describer::*;

#[derive(Debug)]
pub struct InferAndApplyOutput {
    pub sql_schema: SqlSchema,
    pub migration_output: MigrationStepsResultOutput,
}

pub trait MigrationStepsResultOutputExt {
    fn describe_steps(&self) -> Vec<&String>;
}

impl MigrationStepsResultOutputExt for MigrationStepsResultOutput {
    fn describe_steps(&self) -> Vec<&String> {
        self.database_steps
            .iter()
            .map(|step| step.step.as_object().unwrap().keys().next().unwrap())
            .collect()
    }
}
