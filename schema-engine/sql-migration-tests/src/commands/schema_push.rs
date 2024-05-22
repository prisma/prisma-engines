use colored::Colorize;
use schema_core::{
    commands::schema_push, json_rpc::types::*, schema_connector::SchemaConnector, CoreError, CoreResult,
};
use std::time::Duration;
use std::{borrow::Cow, fmt::Debug};
use tracing_futures::Instrument;

pub struct SchemaPush<'a> {
    api: &'a mut dyn SchemaConnector,
    files: Vec<SchemaContainer>,
    force: bool,
    /// Purely for logging diagnostics.
    migration_id: Option<&'a str>,
    // In eventually-consistent systems, we might need to wait for a while before the system refreshes
    max_ddl_refresh_delay: Option<Duration>,
}

impl<'a> SchemaPush<'a> {
    pub fn new(api: &'a mut dyn SchemaConnector, files: &[(&str, &str)], max_refresh_delay: Option<Duration>) -> Self {
        SchemaPush {
            api,
            files: files
                .iter()
                .map(|(path, content)| SchemaContainer {
                    path: path.to_string(),
                    content: content.to_string(),
                })
                .collect(),
            force: false,
            migration_id: None,
            max_ddl_refresh_delay: max_refresh_delay,
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

    fn send_impl(self) -> CoreResult<SchemaPushAssertion> {
        let input = SchemaPushInput {
            schema: SchemasContainer { files: self.files },
            force: self.force,
        };

        let fut = schema_push(input, self.api)
            .instrument(tracing::info_span!("SchemaPush", migration_id = ?self.migration_id));

        let output = test_setup::runtime::run_with_thread_local_runtime(fut)?;

        if let Some(delay) = self.max_ddl_refresh_delay {
            std::thread::sleep(delay);
        }

        Ok(SchemaPushAssertion {
            result: output,
            context: None,
            description: None,
        })
    }

    /// Execute the command and expect it to succeed.
    #[track_caller]
    pub fn send(self) -> SchemaPushAssertion {
        self.send_impl().unwrap()
    }

    /// Execute the command and expect it to fail, returning the error.
    #[track_caller]
    pub fn send_unwrap_err(self) -> CoreError {
        self.send_impl().unwrap_err()
    }
}

pub struct SchemaPushAssertion {
    pub(super) result: SchemaPushOutput,
    pub(super) context: Option<String>,
    pub(super) description: Option<String>,
}

impl Debug for SchemaPushAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.result.fmt(f)
    }
}

impl SchemaPushAssertion {
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn print_context(&self) {
        match &self.context {
            Some(context) => println!("Test failure with context <{}>", context.red()),
            None => {}
        }
        match &self.description {
            Some(description) => println!("{}: {}", "Description".bold(), description.italic()),
            None => {}
        }
    }

    /// Asserts that the command produced no warning and no unexecutable migration message.
    #[track_caller]
    pub fn assert_green(self) -> Self {
        self.assert_no_warning().assert_executable()
    }

    #[track_caller]
    pub fn assert_no_warning(self) -> Self {
        if !self.result.warnings.is_empty() {
            self.print_context();
            println!(
                "Expected {} warnings but got {}.",
                "no".bold(),
                format!("{}", self.result.warnings.len()).red()
            );
            println!("\nWarnings that were {}:", "not expected".bold());
            self.result.warnings.iter().for_each(|found| {
                println!("\t - {}", found.red());
            });

            panic!();
        }

        self
    }

    pub fn assert_warnings(self, warnings: &[Cow<'_, str>]) -> Self {
        let mut good = Vec::new();
        let mut expected_and_not_found = Vec::new();
        let mut found_and_not_expected: Vec<Cow<'_, str>> = Vec::new();

        warnings.iter().for_each(|expected| {
            if self.result.warnings.iter().any(|found| found == expected) {
                good.push(expected);
            } else {
                expected_and_not_found.push(expected);
            }
        });

        self.result.warnings.iter().for_each(|found| {
            if good.iter().any(|g| *g == (*found).as_str()) {
            } else {
                found_and_not_expected.push(found.into());
            }
        });
        if !(expected_and_not_found.is_empty() && found_and_not_expected.is_empty()) {
            self.print_context();
            println!(
                "Expected {} warnings but got {}.",
                format!("{}", warnings.len()).green(),
                format!("{}", self.result.warnings.len()).red()
            );

            println!("\nExpected warnings that were {}:", "not found".bold());
            expected_and_not_found.iter().for_each(|expected| {
                println!("\t - {}", expected.red());
            });

            println!("\nFound warnings that were {}:", "not expected".bold());
            found_and_not_expected.iter().for_each(|found| {
                println!("\t - {}", found.yellow());
            });

            println!("\nWarnings that were {}:", "found and expected".bold());
            good.iter().for_each(|good| {
                println!("\t - {good}");
            });

            panic!();
        }

        self
    }

    #[track_caller]
    pub fn assert_no_steps(self) -> Self {
        if self.result.executed_steps != 0 {
            self.print_context();
            println!(
                "\nTest failure {}: expected {} but got {} steps.",
                "assert_has_executed_steps".bold(),
                "0".green(),
                format!("{}", self.result.executed_steps).red(),
            );

            panic!();
        }

        self
    }

    pub fn assert_has_executed_steps(self) -> Self {
        if self.result.executed_steps == 0 {
            self.print_context();
            println!(
                "\nTest failure {}: expected {} but got {} steps.",
                "assert_has_executed_steps".bold(),
                ">0".green(),
                "0".red(),
            );

            panic!();
        }

        self
    }

    #[track_caller]
    pub fn assert_executable(self) -> Self {
        if !self.result.unexecutable.is_empty() {
            println!("\nExpected no unexecutable errors in {}", "assert_executable".bold());
            self.result.unexecutable.iter().for_each(|unexecutable| {
                println!("\t - {}", unexecutable.red());
            });

            panic!();
        }

        self
    }

    pub fn assert_unexecutable(self, expected_messages: &[String]) -> Self {
        let mut good = Vec::new();
        let mut expected_and_not_found = Vec::new();
        let mut found_and_not_expected: Vec<Cow<'_, str>> = Vec::new();

        expected_messages.iter().for_each(|expected| {
            if self.result.unexecutable.iter().any(|found| found == expected) {
                good.push(expected);
            } else {
                expected_and_not_found.push(expected);
            }
        });

        self.result.unexecutable.iter().for_each(|found| {
            if good.iter().any(|g| *g == (*found).as_str()) {
            } else {
                found_and_not_expected.push(found.into());
            }
        });
        if !(expected_and_not_found.is_empty() && found_and_not_expected.is_empty()) {
            self.print_context();
            println!(
                "Expected {} errors but got {}.",
                format!("{}", expected_messages.len()).green(),
                format!("{}", self.result.unexecutable.len()).red()
            );

            println!("\nExpected errors that were {}:", "not found".bold());
            expected_and_not_found.iter().for_each(|expected| {
                println!("\t - {}", expected.red());
            });

            println!("\nFound errors that were {}:", "not expected".bold());
            found_and_not_expected.iter().for_each(|found| {
                println!("\t - {}", found.yellow());
            });

            println!("\nErrors that were {}:", "found and expected".bold());
            good.iter().for_each(|good| {
                println!("\t - {good}");
            });

            panic!();
        }

        self
    }

    pub fn expect_unexecutable(self, expectation: expect_test::Expect) -> Self {
        expectation.assert_debug_eq(&self.result.unexecutable);
        self
    }
}
