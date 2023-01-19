use crate::{ConnectorTag, ConnectorTagInterface, TestError, TestResult};
use serde::Deserialize;
use std::{convert::TryFrom, env, fs::File, io::Read, path::PathBuf};

static TEST_CONFIG_FILE_NAME: &str = ".test_config";

/// The central test configuration.
#[derive(Debug, Default, Deserialize)]
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
    #[serde(rename = "version")]
    connector_version: Option<String>,

    /// Indicates whether or not the tests are running in CI context.
    /// Env key: `BUILDKITE`
    #[serde(default)]
    is_ci: bool,
}

impl TestConfig {
    /// Loads a configuration. File-based config has precedence over env config.
    pub(crate) fn load() -> TestResult<Self> {
        let config = Self::from_file().or_else(Self::from_env);

        match config {
            Some(config) => {
                config.validate()?;
                config.log_info();

                Ok(config)
            }
            None => Err(TestError::ConfigError(
                "Unable to load config from file or env.".to_owned(),
            )),
        }
    }

    fn log_info(&self) {
        println!("******************************");
        println!("* Test run information:");
        println!("* Runner: {}", self.runner);
        println!(
            "* Connector: {} {}",
            self.connector,
            self.connector_version.as_ref().unwrap_or(&"".to_owned())
        );
        println!("* CI? {}", self.is_ci);
        println!("******************************");
    }

    fn from_env() -> Option<Self> {
        let runner = std::env::var("TEST_RUNNER").ok();
        let connector = std::env::var("TEST_CONNECTOR").ok();
        let connector_version = std::env::var("TEST_CONNECTOR_VERSION").ok();

        // Just care for a set value for now.
        let is_ci = std::env::var("BUILDKITE").is_ok();

        match (runner, connector) {
            (Some(runner), Some(connector)) => Some(Self {
                runner,
                connector,
                connector_version,
                is_ci,
            }),
            _ => None,
        }
    }

    fn from_file() -> Option<Self> {
        let current_dir = env::current_dir().ok();
        let workspace_root = std::env::var("WORKSPACE_ROOT").ok().map(PathBuf::from);

        current_dir
            .and_then(|path| Self::try_path(config_path(path)))
            .or_else(|| workspace_root.and_then(|path| Self::try_path(config_path(path))))
    }

    fn try_path(path: PathBuf) -> Option<Self> {
        File::open(path).ok().and_then(|mut f| {
            let mut config = String::new();

            f.read_to_string(&mut config)
                .ok()
                .and_then(|_| serde_json::from_str(&config).ok())
        })
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

fn config_path(mut path: PathBuf) -> PathBuf {
    path.push(TEST_CONFIG_FILE_NAME);
    path
}
