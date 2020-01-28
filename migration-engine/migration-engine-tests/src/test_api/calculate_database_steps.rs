use migration_connector::MigrationStep;
use migration_core::{
    api::GenericApi,
    commands::{CalculateDatabaseStepsInput, MigrationStepsResultOutput},
};
pub struct CalculateDatabaseSteps<'a> {
    api: &'a dyn GenericApi,
    assume_to_be_applied: Option<Vec<MigrationStep>>,
    steps_to_apply: Option<Vec<MigrationStep>>,
}

impl<'a> CalculateDatabaseSteps<'a> {
    pub fn new(api: &'a dyn GenericApi) -> Self {
        CalculateDatabaseSteps {
            api,
            assume_to_be_applied: None,
            steps_to_apply: None,
        }
    }

    pub fn assume_to_be_applied(mut self, assume_to_be_applied: Option<Vec<MigrationStep>>) -> Self {
        self.assume_to_be_applied = assume_to_be_applied;

        self
    }

    pub fn steps_to_apply(mut self, steps_to_apply: Option<Vec<MigrationStep>>) -> Self {
        self.steps_to_apply = steps_to_apply;

        self
    }

    pub async fn send(self) -> anyhow::Result<MigrationStepsResultOutput> {
        let input = CalculateDatabaseStepsInput {
            assume_to_be_applied: self.assume_to_be_applied,
            steps_to_apply: self.steps_to_apply.unwrap_or_else(Vec::new),
        };

        Ok(self.api.calculate_database_steps(&input).await?)
    }

    pub async fn send_assert(self) -> anyhow::Result<CalculateDatabaseStepsAssertion<'a>> {
        let api = self.api;
        let result = self.send().await?;

        Ok(super::infer_apply::InferApplyAssertion { _api: api, result })
    }
}

pub type CalculateDatabaseStepsAssertion<'a> = super::infer_apply::InferApplyAssertion<'a>;
