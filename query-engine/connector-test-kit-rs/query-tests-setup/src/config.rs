use crate::TestResult;

/// A collection of configuration done via env.
#[derive(Debug)]
pub struct EnvConfig {
    /// TEST_RUNNER
    runner: String,

    /// TEST_CONNECTOR
    connector: String,
}

impl EnvConfig {
    pub fn load() -> TestResult<Self> {
        let connector = std::env::var("TEST_CONNECTOR")?;
        let runner = std::env::var("TEST_RUNNER")?;

        Ok(Self { connector, runner })
    }

    pub fn runner(&self) -> &str {
        self.runner.as_str()
    }

    pub fn connector(&self) -> &str {
        self.connector.as_str()
    }
}
