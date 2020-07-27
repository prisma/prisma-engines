use super::super::assertions::AssertionResult;
use migration_core::{
    api::GenericApi,
    commands::{SchemaPushInput, SchemaPushOutput},
};
use std::borrow::Cow;

pub struct SchemaPush<'a> {
    api: &'a dyn GenericApi,
    schema: String,
    force: bool,
}

impl<'a> SchemaPush<'a> {
    pub fn new(api: &'a dyn GenericApi, schema: String) -> Self {
        SchemaPush {
            api,
            schema,
            force: false,
        }
    }

    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub async fn send(self) -> anyhow::Result<SchemaPushAssertion<'a>> {
        let input = SchemaPushInput {
            schema: self.schema,
            force: self.force,
        };

        let output = self.api.schema_push(&input).await?;

        Ok(SchemaPushAssertion {
            result: output,
            _api: self.api,
        })
    }
}

pub struct SchemaPushAssertion<'a> {
    pub(super) result: SchemaPushOutput,
    pub(super) _api: &'a dyn GenericApi,
}

impl<'a> SchemaPushAssertion<'a> {
    /// Asserts that the command produced no warning and no unexecutable migration message.
    pub fn assert_green(self) -> AssertionResult<Self> {
        self.assert_no_warning()?.assert_executable()
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
                self.result.warnings.get(idx).map(String::as_str)
            );
        }

        Ok(self)
    }

    pub fn assert_no_steps(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.result.executed_steps == 0,
            "Assertion failed. Executed steps should be zero, but found {:#?}",
            self.result.executed_steps
        );

        Ok(self)
    }

    pub fn assert_has_executed_steps(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.result.executed_steps != 0,
            "Assertion failed. Executed steps should be not zero.",
        );

        Ok(self)
    }

    pub fn assert_executable(self) -> AssertionResult<Self> {
        assert!(self.result.unexecutable.is_empty());

        Ok(self)
    }

    pub fn assert_unexecutable(self, expected_messages: &[String]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.result.unexecutable.len() == expected_messages.len(),
            "Expected {} unexecutable step errors, got {}.\n({:#?})",
            expected_messages.len(),
            self.result.unexecutable.len(),
            self.result.unexecutable,
        );

        for (expected, actual) in expected_messages
            .iter()
            .zip(self.result.unexecutable.iter().map(String::as_str))
        {
            assert_eq!(actual, expected);
        }

        Ok(self)
    }
}
