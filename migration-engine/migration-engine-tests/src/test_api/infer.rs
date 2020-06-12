use super::super::unique_migration_id;
use crate::AssertionResult;
use migration_connector::MigrationStep;
use migration_core::{
    api::GenericApi,
    commands::{AppliedMigration, InferMigrationStepsInput, MigrationStepsResultOutput},
};

pub struct Infer<'a> {
    pub(super) api: &'a dyn GenericApi,
    pub(super) assume_to_be_applied: Option<Vec<MigrationStep>>,
    pub(super) assume_applied_migrations: Option<Vec<AppliedMigration>>,
    pub(super) datamodel: String,
    pub(super) migration_id: Option<String>,
}

impl<'a> Infer<'a> {
    pub fn new(api: &'a dyn GenericApi, dm: impl Into<String>) -> Self {
        Infer {
            api,
            datamodel: dm.into(),
            assume_to_be_applied: None,
            assume_applied_migrations: None,
            migration_id: None,
        }
    }

    pub fn migration_id(mut self, migration_id: Option<impl Into<String>>) -> Self {
        self.migration_id = migration_id.map(Into::into);
        self
    }

    pub fn assume_to_be_applied(mut self, assume_to_be_applied: Option<Vec<MigrationStep>>) -> Self {
        self.assume_to_be_applied = assume_to_be_applied;
        self
    }

    pub fn assume_applied_migrations(mut self, assume_applied_migrations: Option<Vec<AppliedMigration>>) -> Self {
        self.assume_applied_migrations = assume_applied_migrations;
        self
    }

    pub async fn send_assert(self) -> anyhow::Result<InferAssertion<'a>> {
        let api = self.api;
        let result = self.send().await?;

        Ok(InferAssertion { result, _api: api })
    }

    pub async fn send(self) -> Result<MigrationStepsResultOutput, anyhow::Error> {
        let migration_id = self.migration_id.unwrap_or_else(unique_migration_id);

        let input = InferMigrationStepsInput {
            assume_to_be_applied: Some(self.assume_to_be_applied.unwrap_or_else(Vec::new)),
            assume_applied_migrations: self.assume_applied_migrations,
            datamodel: self.datamodel,
            migration_id,
        };

        let output = self.api.infer_migration_steps(&input).await?;

        assert!(
            output.general_errors.is_empty(),
            format!("InferMigration returned unexpected errors: {:?}", output.general_errors)
        );

        Ok(output)
    }
}

pub struct InferAssertion<'a> {
    result: MigrationStepsResultOutput,
    _api: &'a dyn GenericApi,
}

impl<'a> InferAssertion<'a> {
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

    pub fn assert_no_error(self) -> AssertionResult<Self> {
        assert!(self.result.general_errors.is_empty());

        Ok(self)
    }

    pub fn assert_executable(self) -> AssertionResult<Self> {
        assert!(self.result.unexecutable_migrations.is_empty());

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
}
