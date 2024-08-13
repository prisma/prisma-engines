use schema_core::{commands::evaluate_data_loss, json_rpc::types::*, schema_connector::SchemaConnector, CoreResult};
use std::borrow::Cow;
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See EvaluateDataLoss::send()"]
pub struct EvaluateDataLoss<'a> {
    api: &'a mut dyn SchemaConnector,
    migrations_directory: &'a TempDir,
    files: Vec<SchemaContainer>,
}

impl<'a> EvaluateDataLoss<'a> {
    pub fn new<'b>(
        api: &'a mut dyn SchemaConnector,
        migrations_directory: &'a TempDir,
        files: &[(&'b str, &'b str)],
    ) -> Self {
        EvaluateDataLoss {
            api,
            migrations_directory,
            files: files
                .iter()
                .map(|(path, content)| SchemaContainer {
                    path: path.to_string(),
                    content: content.to_string(),
                })
                .collect(),
        }
    }

    fn send_impl(self) -> CoreResult<EvaluateDataLossAssertion<'a>> {
        let fut = evaluate_data_loss(
            EvaluateDataLossInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
                schema: SchemasContainer { files: self.files },
            },
            self.api,
        );
        let output = test_setup::runtime::run_with_thread_local_runtime(fut)?;

        Ok(EvaluateDataLossAssertion {
            output,
            _migrations_directory: self.migrations_directory,
        })
    }

    #[track_caller]
    pub fn send(self) -> EvaluateDataLossAssertion<'a> {
        self.send_impl().unwrap()
    }
}

pub struct EvaluateDataLossAssertion<'a> {
    output: EvaluateDataLossOutput,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for EvaluateDataLossAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvaluateDataLossAssertion").finish()
    }
}

impl<'a> EvaluateDataLossAssertion<'a> {
    #[track_caller]
    pub fn assert_steps_count(self, count: u32) -> Self {
        assert!(
            self.output.migration_steps == count,
            "Assertion failed. Expected evaluateDataLoss to return {} steps, found {}",
            count,
            self.output.migration_steps,
        );

        self
    }

    pub fn assert_warnings(self, warnings: &[Cow<'_, str>]) -> Self {
        assert_eq!(
            self.output.warnings.len(),
            warnings.len(),
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

        self
    }

    pub fn assert_warnings_with_indices(self, warnings: &[(Cow<'_, str>, u32)]) -> Self {
        assert!(
            self.output.warnings.len() == warnings.len(),
            "Expected {} warnings, got {}.\n{:#?}",
            warnings.len(),
            self.output.warnings.len(),
            self.output.warnings
        );

        let descriptions: Vec<(Cow<'_, str>, u32)> = self
            .output
            .warnings
            .iter()
            .map(|warning| (warning.message.as_str().into(), warning.step_index))
            .collect();

        assert_eq!(descriptions, warnings);

        self
    }

    pub fn assert_unexecutable(self, unexecutable_steps: &[Cow<'_, str>]) -> Self {
        assert!(
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

        self
    }

    pub fn assert_unexecutables_with_indices(self, unexecutables: &[(Cow<'_, str>, u32)]) -> Self {
        assert!(
            self.output.unexecutable_steps.len() == unexecutables.len(),
            "Expected {} unexecutables, got {}.\n{:#?}",
            unexecutables.len(),
            self.output.unexecutable_steps.len(),
            self.output.unexecutable_steps
        );

        let descriptions: Vec<(Cow<'_, str>, u32)> = self
            .output
            .unexecutable_steps
            .iter()
            .map(|warning| (warning.message.as_str().into(), warning.step_index))
            .collect();

        assert_eq!(descriptions, unexecutables);
        self
    }

    pub fn into_output(self) -> EvaluateDataLossOutput {
        self.output
    }
}
