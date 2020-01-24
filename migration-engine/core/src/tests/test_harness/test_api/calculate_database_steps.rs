use crate::{
    api::GenericApi,
    commands::{AppliedMigration, CalculateDatabaseStepsInput, MigrationStepsResultOutput},
};
use migration_connector::MigrationStep;
pub(crate) struct CalculateDatabaseSteps<'a> {
    api: &'a dyn GenericApi,
    assume_to_be_applied: Option<Vec<MigrationStep>>,
    assume_applied_migrations: Option<Vec<AppliedMigration>>,
    steps_to_apply: Option<Vec<MigrationStep>>,
}

impl<'a> CalculateDatabaseSteps<'a> {
    pub(crate) fn new(api: &'a dyn GenericApi) -> Self {
        CalculateDatabaseSteps {
            api,
            assume_applied_migrations: None,
            assume_to_be_applied: None,
            steps_to_apply: None,
        }
    }

    pub(crate) fn assume_applied_migrations(
        mut self,
        assume_applied_migrations: Option<Vec<AppliedMigration>>,
    ) -> Self {
        self.assume_applied_migrations = assume_applied_migrations;

        self
    }

    pub(crate) fn assume_to_be_applied(mut self, assume_to_be_applied: Option<Vec<MigrationStep>>) -> Self {
        self.assume_to_be_applied = assume_to_be_applied;

        self
    }

    pub(crate) fn steps_to_apply(mut self, steps_to_apply: Option<Vec<MigrationStep>>) -> Self {
        self.steps_to_apply = steps_to_apply;

        self
    }

    pub(crate) async fn send(self) -> anyhow::Result<MigrationStepsResultOutput> {
        let input = CalculateDatabaseStepsInput {
            assume_to_be_applied: self.assume_to_be_applied,
            assume_applied_migrations: self.assume_applied_migrations,
            steps_to_apply: self.steps_to_apply.unwrap_or_else(Vec::new),
        };

        Ok(self.api.calculate_database_steps(&input).await?)
    }
}
