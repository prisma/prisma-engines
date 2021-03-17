use std::convert::TryFrom;

use crate::{ConnectorTag, ConnectorTagInterface, TestError, TestResult};

/// The central test configuration.
#[derive(Debug, Default)]
pub struct TestConfig {
    /// The test runner to use for the tests.
    /// Env key: `TEST_RUNNER`
    runner: String,

    /// The connector that tests should run for.
    /// Env key: `TEST_CONNECTOR`
    connector: String,

    /// The connector version tests should run for.
    /// If the test connector is versioned, this option is required.
    /// Env key: `TEST_CONNECTOR_VERSION`
    connector_version: Option<String>,

    /// Indicates whether or not the tests are running in CI context.
    /// Env key: `BUILDKITE`
    is_ci: bool,
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

        // Just care for a set value for now.
        let is_ci = match std::env::var("BUILDKITE") {
            Ok(_) => true,
            Err(_) => false,
        };

        Ok(Self {
            runner,
            connector,
            connector_version,
            is_ci,
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

        if self.test_connector_tag()?.is_versioned() && self.connector_version.is_none() {
            return Err(TestError::config_error(
                "The current test connector requires a version to be set to run.",
            ));
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

    pub fn is_ci(&self) -> bool {
        self.is_ci
    }

    pub fn test_connector_tag(&self) -> TestResult<ConnectorTag> {
        ConnectorTag::try_from((self.connector(), self.connector_version()))
    }
}
