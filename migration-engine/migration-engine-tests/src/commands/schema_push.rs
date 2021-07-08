use migration_core::{
    commands::{SchemaPushInput, SchemaPushOutput},
    CoreError, CoreResult, GenericApi,
};
use std::{borrow::Cow, fmt::Debug};
use tracing_futures::Instrument;

pub struct SchemaPush<'a> {
    api: &'a dyn GenericApi,
    schema: String,
    force: bool,
    /// Purely for logging diagnostics.
    migration_id: Option<&'a str>,
    rt: &'a tokio::runtime::Runtime,
}

impl<'a> SchemaPush<'a> {
    pub fn new(api: &'a dyn GenericApi, schema: String, rt: &'a tokio::runtime::Runtime) -> Self {
        SchemaPush {
            api,
            schema,
            force: false,
            migration_id: None,
            rt,
        }
    }

    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub fn migration_id(mut self, migration_id: Option<&'a str>) -> Self {
        self.migration_id = migration_id;
        self
    }

    fn send_impl(self) -> CoreResult<SchemaPushAssertion<'a>> {
        let input = SchemaPushInput {
            schema: self.schema,
            force: self.force,
            assume_empty: false,
        };

        let fut = self
            .api
            .schema_push(&input)
            .instrument(tracing::info_span!("SchemaPush", migration_id = ?self.migration_id));

        let output = self.rt.block_on(fut)?;

        Ok(SchemaPushAssertion {
            result: output,
            _api: self.api,
        })
    }

    /// Execute the command and expect it to succeed.
    #[track_caller]
    pub fn send(self) -> SchemaPushAssertion<'a> {
        self.send_impl().unwrap()
    }

    /// Execute the command and expect it to fail, returning the error.
    #[track_caller]
    pub fn send_unwrap_err(self) -> CoreError {
        self.send_impl().unwrap_err()
    }
}

pub struct SchemaPushAssertion<'a> {
    pub(super) result: SchemaPushOutput,
    pub(super) _api: &'a dyn GenericApi,
}

impl Debug for SchemaPushAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.result.fmt(f)
    }
}

impl<'a> SchemaPushAssertion<'a> {
    /// Asserts that the command produced no warning and no unexecutable migration message.
    #[track_caller]
    pub fn assert_green_bang(self) -> Self {
        self.assert_no_warning().assert_executable()
    }

    pub fn assert_no_warning(self) -> Self {
        assert!(
            self.result.warnings.is_empty(),
            "Assertion failed. Expected no warning, got {:?}",
            self.result.warnings
        );

        self
    }

    pub fn assert_warnings(self, warnings: &[Cow<'_, str>]) -> Self {
        assert!(
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

        self
    }

    #[track_caller]
    pub fn assert_no_steps(self) -> Self {
        assert!(
            self.result.executed_steps == 0,
            "Assertion failed. Executed steps should be zero, but found {:#?}",
            self.result.executed_steps,
        );
        self
    }

    pub fn assert_has_executed_steps(self) -> Self {
        assert!(
            self.result.executed_steps != 0,
            "Assertion failed. Executed steps should be not zero.",
        );
        self
    }

    #[track_caller]
    pub fn assert_executable(self) -> Self {
        assert!(
            self.result.unexecutable.is_empty(),
            "Expected an executable migration, got following: {:?}",
            self.result.unexecutable
        );

        self
    }

    pub fn assert_unexecutable(self, expected_messages: &[String]) -> Self {
        assert_eq!(
            self.result.unexecutable.len(),
            expected_messages.len(),
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

        self
    }
}
