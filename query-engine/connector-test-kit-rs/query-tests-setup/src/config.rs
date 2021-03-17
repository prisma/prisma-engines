use crate::{TestError, TestResult};

/// A collection of configuration done via env.
/// `runner`: The test runner to use for the tests.
/// `connector`: The connector that tests should run for.
/// `connector_version`: The connector version tests should run for.
///                      If no version is given, all tests of that family will be run.
#[derive(Debug, Default)]
pub struct TestConfig {
    /// TEST_RUNNER
    runner: String,

    /// TEST_CONNECTOR
    connector: String,

    /// TEST_CONNECTOR_VERSION
    connector_version: Option<String>,
}

impl TestConfig {
    /// Loads a configuration. File-based config has precedence over env config.
    pub fn load() -> TestResult<Self> {
        let config = Self::from_file()?.merge_left(Self::from_env()?);

        config.validate()?;
        Ok(config)
    }

    fn from_env() -> TestResult<Self> {
        let runner = std::env::var("TEST_RUNNER").unwrap_or_else(|_| String::new());
        let connector = std::env::var("TEST_CONNECTOR").unwrap_or_else(|_| String::new());
        let connector_version = std::env::var("TEST_CONNECTOR_VERSION").ok();

        Ok(Self {
            runner,
            connector,
            connector_version,
        })
    }

    fn from_file() -> TestResult<Self> {
        // todo
        Ok(Self::default())
    }

    fn validate(&self) -> TestResult<()> {
        if self.runner.is_empty() {
            return Err(TestError::config_error("A test runner is required but was not set."));
        }

        if self.connector.is_empty() {
            return Err(TestError::config_error("A test connector is required but was not set."));
        }

        Ok(())
    }

    /// Merges two configurations, retaining any non-empty value that is already set on `self`.
    /// Overwrites empty values with values from `other`.
    fn merge_left(mut self, other: Self) -> Self {
        if self.runner.is_empty() {
            self.runner = other.runner;
        }

        if self.connector.is_empty() {
            self.connector = other.connector;
        }

        if self.connector_version.is_none() {
            self.connector_version = other.connector_version;
        }

        self
    }

    pub fn runner(&self) -> &str {
        self.runner.as_str()
    }

    pub fn connector(&self) -> &str {
        self.connector.as_str()
    }

    pub fn connector_version(&self) -> Option<&str> {
        self.connector_version.as_ref().map(AsRef::as_ref)
    }
}
