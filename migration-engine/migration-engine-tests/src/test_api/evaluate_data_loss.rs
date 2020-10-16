use crate::AssertionResult;
use migration_core::{
    commands::{EvaluateDataLossInput, EvaluateDataLossOutput},
    GenericApi,
};
use std::borrow::Cow;
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See EvaluateDataLoss::send()"]
pub struct EvaluateDataLoss<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
    prisma_schema: String,
}

impl<'a> EvaluateDataLoss<'a> {
    pub fn new(api: &'a dyn GenericApi, migrations_directory: &'a TempDir, prisma_schema: String) -> Self {
        EvaluateDataLoss {
            api,
            migrations_directory,
            prisma_schema,
        }
    }

    pub async fn send(self) -> anyhow::Result<EvaluateDataLossAssertion<'a>> {
        let output = self
            .api
            .evaluate_data_loss(&EvaluateDataLossInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
                prisma_schema: self.prisma_schema,
            })
            .await?;

        Ok(EvaluateDataLossAssertion {
            output,
            _api: self.api,
            _migrations_directory: self.migrations_directory,
        })
    }
}

pub struct EvaluateDataLossAssertion<'a> {
    output: EvaluateDataLossOutput,
    _api: &'a dyn GenericApi,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for EvaluateDataLossAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvaluateDataLossAssertion").finish()
    }
}

impl<'a> EvaluateDataLossAssertion<'a> {
    pub fn assert_steps_count(self, count: usize) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.output.migration_steps.len() == count,
            "Assertion failed. Expected evaluateDataLoss to return {} steps, found {}.\n{:?}",
            count,
            self.output.migration_steps.len(),
            self.output.migration_steps,
        );

        Ok(self)
    }

    pub fn assert_warnings(self, warnings: &[Cow<'_, str>]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.output.warnings.len() == warnings.len(),
            "Expected {} warnings, got {}.\n{:#?}",
            warnings.len(),
            self.output.warnings.len(),
            self.output.warnings
        );

        let descriptions: Vec<Cow<'_, str>> = self
            .output
            .warnings
            .iter()
            .map(|warning| warning.message.as_str().into())
            .collect();

        assert_eq!(descriptions, warnings);

        Ok(self)
    }

    pub fn assert_warnings_with_indices(self, warnings: &[(Cow<'_, str>, usize)]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.output.warnings.len() == warnings.len(),
            "Expected {} warnings, got {}.\n{:#?}",
            warnings.len(),
            self.output.warnings.len(),
            self.output.warnings
        );

        let descriptions: Vec<(Cow<'_, str>, usize)> = self
            .output
            .warnings
            .iter()
            .map(|warning| (warning.message.as_str().into(), warning.step_index))
            .collect();

        assert_eq!(descriptions, warnings);

        Ok(self)
    }

    pub fn assert_unexecutable(self, unexecutable_steps: &[Cow<'_, str>]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.output.unexecutable_steps.len() == unexecutable_steps.len(),
            "Expected {} unexecutable_steps, got {}.\n{:#?}",
            unexecutable_steps.len(),
            self.output.unexecutable_steps.len(),
            self.output.unexecutable_steps
        );

        let descriptions: Vec<Cow<'_, str>> = self
            .output
            .unexecutable_steps
            .iter()
            .map(|warning| warning.message.as_str().into())
            .collect();

        assert_eq!(descriptions, unexecutable_steps);

        Ok(self)
    }

    pub fn assert_unexecutables_with_indices(self, unexecutables: &[(Cow<'_, str>, usize)]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.output.unexecutable_steps.len() == unexecutables.len(),
            "Expected {} unexecutables, got {}.\n{:#?}",
            unexecutables.len(),
            self.output.unexecutable_steps.len(),
            self.output.unexecutable_steps
        );

        let descriptions: Vec<(Cow<'_, str>, usize)> = self
            .output
            .unexecutable_steps
            .iter()
            .map(|warning| (warning.message.as_str().into(), warning.step_index))
            .collect();

        assert_eq!(descriptions, unexecutables);

        Ok(self)
    }

    pub fn into_output(self) -> EvaluateDataLossOutput {
        self.output
    }
}
