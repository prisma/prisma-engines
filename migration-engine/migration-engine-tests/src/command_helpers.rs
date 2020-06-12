use migration_connector::PrettyDatabaseMigrationStep;
use migration_core::commands::*;
use sql_migration_connector::SqlMigrationStep;
use sql_schema_describer::*;

#[derive(Debug)]
pub struct InferAndApplyOutput {
    pub sql_schema: SqlSchema,
    pub migration_output: MigrationStepsResultOutput,
}

impl InferAndApplyOutput {
    pub fn sql_migration(&self) -> Vec<SqlMigrationStep> {
        self.migration_output.sql_migration()
    }
}

pub trait MigrationStepsResultOutputExt {
    fn sql_migration(&self) -> Vec<SqlMigrationStep>;
}

impl MigrationStepsResultOutputExt for MigrationStepsResultOutput {
    fn sql_migration(&self) -> Vec<SqlMigrationStep> {
        self.database_steps
            .iter()
            .map(|pretty_step| serde_json::from_value(pretty_step.step.clone()).unwrap())
            .collect()
    }
}
