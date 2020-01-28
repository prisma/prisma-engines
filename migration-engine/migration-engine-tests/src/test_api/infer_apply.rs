use super::super::{assertions::AssertionResult, unique_migration_id};
use migration_core::{
    api::GenericApi,
    commands::{ApplyMigrationInput, InferMigrationStepsInput, MigrationStepsResultOutput},
};

pub struct InferApply<'a> {
    api: &'a dyn GenericApi,
    schema: &'a str,
    migration_id: Option<String>,
    force: Option<bool>,
}

impl<'a> InferApply<'a> {
    pub fn new(api: &'a dyn GenericApi, schema: &'a str) -> Self {
        InferApply {
            api,
            schema,
            migration_id: None,
            force: None,
        }
    }

    pub fn force(mut self, force: Option<bool>) -> Self {
        self.force = force;
        self
    }

    pub fn migration_id(mut self, migration_id: Option<impl Into<String>>) -> Self {
        self.migration_id = migration_id.map(Into::into);
        self
    }

    pub async fn send(self) -> Result<MigrationStepsResultOutput, anyhow::Error> {
        let migration_id = self.migration_id.map(Into::into).unwrap_or_else(unique_migration_id);

        let input = InferMigrationStepsInput {
            migration_id: migration_id.clone(),
            datamodel: self.schema.to_owned(),
            assume_to_be_applied: Some(Vec::new()),
            assume_applied_migrations: None,
        };

        let steps = self.api.infer_migration_steps(&input).await?.datamodel_steps;

        let input = ApplyMigrationInput {
            migration_id,
            steps,
            force: self.force,
        };

        let migration_output = self.api.apply_migration(&input).await?;

        Ok(migration_output)
    }

    pub async fn send_assert(self) -> Result<InferApplyAssertion<'a>, anyhow::Error> {
        let api = self.api;
        let result = self.send().await?;

        Ok(InferApplyAssertion { result, _api: api })
    }
}

pub struct InferApplyAssertion<'a> {
    pub(super) result: MigrationStepsResultOutput,
    pub(super) _api: &'a dyn GenericApi,
}

impl<'a> InferApplyAssertion<'a> {
    pub fn assert_green(self) -> AssertionResult<Self> {
        assert!(self.result.warnings.is_empty());
        assert!(self.result.general_errors.is_empty());
        assert!(self.result.unexecutable_migrations.is_empty());

        Ok(self)
    }

    pub fn assert_no_warning(self) -> AssertionResult<Self> {
        assert!(self.result.warnings.is_empty());

        Ok(self)
    }

    pub fn assert_no_error(self) -> AssertionResult<Self> {
        assert!(self.result.general_errors.is_empty());

        Ok(self)
    }

    pub fn assert_executable(self) -> AssertionResult<Self> {
        assert!(self.result.unexecutable_migrations.is_empty());

        Ok(self)
    }

    pub fn assert_unexecutable(self, expected_messages: &[String]) -> AssertionResult<Self> {
        assert_eq!(self.result.unexecutable_migrations.len(), expected_messages.len());

        for (expected, actual) in expected_messages.iter().zip(
            self.result
                .unexecutable_migrations
                .iter()
                .map(|w| w.description.as_str()),
        ) {
            assert_eq!(actual, expected);
        }

        Ok(self)
    }

    pub fn into_inner(self) -> MigrationStepsResultOutput {
        self.result
    }
}
