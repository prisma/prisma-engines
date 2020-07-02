use super::super::{assertions::AssertionResult, unique_migration_id};
use migration_core::{
    api::GenericApi,
    commands::{ApplyMigrationInput, InferMigrationStepsInput, MigrationStepsResultOutput},
};
use std::borrow::Cow;

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

    pub async fn send(self) -> Result<InferApplyAssertion<'a>, anyhow::Error> {
        let api = self.api;
        let result = self.send_inner().await?;

        Ok(InferApplyAssertion { result, _api: api })
    }

    pub async fn send_user_facing(self) -> Result<MigrationStepsResultOutput, user_facing_errors::Error> {
        let api = self.api;
        self.send_inner().await.map_err(|err| api.render_error(err))
    }

    pub async fn send_inner(self) -> Result<MigrationStepsResultOutput, migration_core::error::Error> {
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
}

pub struct InferApplyAssertion<'a> {
    pub(super) result: MigrationStepsResultOutput,
    pub(super) _api: &'a dyn GenericApi,
}

impl<'a> InferApplyAssertion<'a> {
    pub fn assert_green(self) -> AssertionResult<Self> {
        self.assert_no_warning()?.assert_no_error()?.assert_executable()
    }

    pub fn assert_no_warning(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.result.warnings.is_empty(),
            "Assertion failed. Expected no warning, got {:?}",
            self.result.warnings
        );

        Ok(self)
    }

    pub fn assert_warnings(self, warnings: &[Cow<'_, str>]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.result.warnings.len() == warnings.len(),
            "Expected {} warnings, got {}.\n{:#?}",
            warnings.len(),
            self.result.warnings.len(),
            self.result.warnings
        );

        for (idx, warning) in warnings.iter().enumerate() {
            assert_eq!(
                Some(warning.as_ref()),
                self.result
                    .warnings
                    .get(idx)
                    .map(|warning| warning.description.as_str())
            );
        }

        Ok(self)
    }

    pub fn assert_no_error(self) -> AssertionResult<Self> {
        assert!(self.result.general_errors.is_empty());

        Ok(self)
    }

    pub fn assert_no_steps(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.result.datamodel_steps.is_empty(),
            "Assertion failed. Datamodel migration steps should be empty, but found {:#?}",
            self.result.datamodel_steps
        );

        anyhow::ensure!(
            self.result.database_steps.is_empty(),
            "Assertion failed. Database migration steps should be empty, but found {:#?}",
            self.result.database_steps
        );

        Ok(self)
    }

    pub fn assert_executable(self) -> AssertionResult<Self> {
        assert!(self.result.unexecutable_migrations.is_empty());

        Ok(self)
    }

    pub fn assert_unexecutable(self, expected_messages: &[String]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.result.unexecutable_migrations.len() == expected_messages.len(),
            "Expected {} unexecutable step errors, got {}.\n({:#?})",
            expected_messages.len(),
            self.result.unexecutable_migrations.len(),
            self.result.unexecutable_migrations,
        );

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
