use crate::{
    CockroachDbConnectorTag, ConnectorTag, ConnectorVersion, MongoDbConnectorTag, MySqlConnectorTag,
    PostgresConnectorTag, SqlServerConnectorTag, SqliteConnectorTag, TestResult, VitessConnectorTag,
};
use log::warn;
use qe_setup::driver_adapters::DriverAdapter;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, env, fs::File, io::Read, path::PathBuf};

static TEST_CONFIG_FILE_NAME: &str = ".test_config";

/// The central test configuration.
/// This struct is a 1:1 mapping to the test config file.
/// After validation, this is used to generate [`TestConfig`]
#[derive(Debug, Deserialize)]
pub struct TestConfigFromSerde {
    /// The connector that tests should run for.
    /// Env key: `TEST_CONNECTOR`
    pub(crate) connector: String,

    /// The connector version tests should run for.
    /// If the test connector is versioned, this option is required.
    /// Env key: `TEST_CONNECTOR_VERSION`
    #[serde(rename = "version")]
    pub(crate) connector_version: Option<String>,

    /// The driver adapter to use when running tests, will be forwarded to the external test
    /// executor by setting the `DRIVER_ADAPTER` env var when spawning the executor process.
    /// Correctness: if set, [`TestConfigFromSerde::external_test_executor`] and
    /// [`TestConfigFromSerde::driver_adapter_config`] must be set as well.
    pub(crate) driver_adapter: DriverAdapter,

    /// The driver adapter configuration to forward as a stringified JSON object to the external
    /// test executor by setting the `DRIVER_ADAPTER_CONFIG` env var when spawning the executor.
    /// Correctness: if set, [`TestConfigFromSerde::driver_adapter`] must be set as well.
    pub(crate) driver_adapter_config: Option<DriverAdapterConfig>,

    /// The maximum number of bind values to use in a query for a driver adapter test runner.
    pub(crate) driver_adapter_max_bind_values: Option<usize>,
}

impl TestConfigFromSerde {
    pub fn test_connector(&self) -> TestResult<(ConnectorTag, ConnectorVersion)> {
        let version = ConnectorVersion::try_from((self.connector.as_str(), self.connector_version.as_deref()))?;
        let tag = match version {
            ConnectorVersion::SqlServer(_) => &SqlServerConnectorTag as ConnectorTag,
            ConnectorVersion::Postgres(_) => &PostgresConnectorTag,
            ConnectorVersion::MySql(_) => &MySqlConnectorTag,
            ConnectorVersion::MongoDb(_) => &MongoDbConnectorTag,
            ConnectorVersion::Sqlite(_) => &SqliteConnectorTag,
            ConnectorVersion::CockroachDb(_) => &CockroachDbConnectorTag,
            ConnectorVersion::Vitess(_) => &VitessConnectorTag,
        };

        Ok((tag, version))
    }

    pub(crate) fn validate(&self) {
        if self.connector.is_empty() {
            exit_with_message("A test connector is required but was not set.");
        }

        match self.test_connector().map(|(_, v)| v) {
            Ok(ConnectorVersion::Vitess(None))
            | Ok(ConnectorVersion::MySql(None))
            | Ok(ConnectorVersion::SqlServer(None))
            | Ok(ConnectorVersion::MongoDb(None))
            | Ok(ConnectorVersion::CockroachDb(None))
            | Ok(ConnectorVersion::Postgres(None))
            | Ok(ConnectorVersion::Sqlite(None)) => {
                exit_with_message("The current test connector requires a version to be set to run.");
            }
            Ok(ConnectorVersion::Vitess(Some(_)))
            | Ok(ConnectorVersion::MySql(Some(_)))
            | Ok(ConnectorVersion::SqlServer(Some(_)))
            | Ok(ConnectorVersion::MongoDb(Some(_)))
            | Ok(ConnectorVersion::CockroachDb(Some(_)))
            | Ok(ConnectorVersion::Postgres(Some(_)))
            | Ok(ConnectorVersion::Sqlite(Some(_))) => (),
            Err(err) => exit_with_message(&err.to_string()),
        }
    }
}

// This struct contains every `driverAdapters`-related configuration entry.
pub(crate) struct WithDriverAdapter {
    /// The driver adapter to use when running tests, will be forwarded to the external test
    /// executor by setting the `DRIVER_ADAPTER` env var when spawning the executor process.
    pub(crate) adapter: DriverAdapter,

    /// The driver adapter configuration to forward as a stringified JSON object to the external
    /// test executor by setting the `DRIVER_ADAPTER_CONFIG` env var when spawning the executor.
    pub(crate) config: Option<DriverAdapterConfig>,

    /// The maximum number of bind values to use in a query for a driver adapter test runner.
    pub(crate) max_bind_values: Option<usize>,
}

impl WithDriverAdapter {
    fn json_stringify_config(&self) -> String {
        self.config.as_ref().map(|cfg| cfg.json_stringify()).unwrap_or_default()
    }
}

pub struct TestConfig {
    pub(crate) connector: String,
    pub(crate) connector_version: Option<String>,
    pub(crate) with_driver_adapter: WithDriverAdapter,
}

impl From<TestConfigFromSerde> for TestConfig {
    fn from(config: TestConfigFromSerde) -> Self {
        config.validate();

        let with_driver_adapter = WithDriverAdapter {
            adapter: config.driver_adapter,
            config: config.driver_adapter_config,
            max_bind_values: config.driver_adapter_max_bind_values,
        };

        Self {
            connector: config.connector,
            connector_version: config.connector_version,
            with_driver_adapter,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct DriverAdapterConfig {
    pub(crate) proxy_url: Option<String>,
}

impl DriverAdapterConfig {
    fn json_stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

const CONFIG_LOAD_FAILED: &str = r####"
=============================================
🔴 Unable to load config from file or env. 🔴
=============================================

ℹ️  How do I fix this? ℹ️

Test config can come from the environment, or a config file.

♻️  Environment variables

Be sure to have WORKSPACE_ROOT set to the root of the prisma-engines
repository.

Set the following vars to denote the connector under test

- TEST_CONNECTOR
- TEST_CONNECTOR_VERSION (optional)

And optionally, to test driver adapters

- EXTERNAL_TEST_EXECUTOR
- DRIVER_ADAPTER
- DRIVER_ADAPTER_CONFIG (optional, not required by all driver adapters)


📁 Config file

Use the Makefile.
"####;

fn exit_with_message(msg: &str) -> ! {
    use std::io::{Write, stderr};
    let stderr = stderr();
    let mut sink = stderr.lock();
    sink.write_all(b"Error in the test configuration:\n").unwrap();
    sink.write_all(msg.as_bytes()).unwrap();
    sink.write_all(b"Aborting test process\n").unwrap();

    std::process::exit(1)
}

impl TestConfig {
    /// Loads a configuration. File-based config has precedence over env config.
    pub(crate) fn load() -> Self {
        // Use `RUST_LOG=warn` to see warnings about config loading.
        env_logger::init();
        let config = match Self::from_file().or_else(Self::from_env) {
            Some(config) => config,
            None => exit_with_message(CONFIG_LOAD_FAILED),
        };

        config.validate();
        config.log_info();

        config
    }

    pub(crate) fn with_driver_adapter(&self) -> &WithDriverAdapter {
        &self.with_driver_adapter
    }

    #[rustfmt::skip]
    fn log_info(&self) {
        println!("******************************");
        println!("* Test run information:");
        println!(
            "* Connector: {} {}",
            self.connector,
            self.connector_version().unwrap_or_default()
        );
        println!("* Driver adapter: {}", self.with_driver_adapter.adapter);
        println!("* Driver adapter config: {}", self.with_driver_adapter.json_stringify_config());
        println!("******************************");
    }

    fn from_env() -> Option<Self> {
        let connector = std::env::var("TEST_CONNECTOR").ok();
        let connector_version = std::env::var("TEST_CONNECTOR_VERSION").ok();

        let driver_adapter = std::env::var("DRIVER_ADAPTER").ok().map(DriverAdapter::from);
        let driver_adapter_config = std::env::var("DRIVER_ADAPTER_CONFIG")
            .map(|config| serde_json::from_str::<DriverAdapterConfig>(config.as_str()).ok())
            .unwrap_or_default();
        let driver_adapter_max_bind_values = std::env::var("DRIVER_ADAPTER_MAX_BIND_VALUES")
            .ok()
            .map(|v| v.parse::<usize>().unwrap());

        Some(Self::from(TestConfigFromSerde {
            connector: connector?,
            connector_version,
            driver_adapter: driver_adapter?,
            driver_adapter_config,
            driver_adapter_max_bind_values,
        }))
    }

    fn from_file() -> Option<Self> {
        Self::workspace_root().and_then(|path| Self::try_path(config_path(path)))
    }

    fn try_path(path: PathBuf) -> Option<Self> {
        File::open(&path)
            .map_err(move |err| {
                warn!("Could not open file {}: {err}", path.display());
                err
            })
            .ok()
            .and_then(|mut f| {
                let mut config = String::new();

                f.read_to_string(&mut config)
                    .map_err(|err| {
                        warn!("Could not read file into string: {err}");
                        err
                    })
                    .ok()
                    .and_then(|_| {
                        serde_json::from_str::<TestConfigFromSerde>(&config)
                            .map_err(|err| {
                                warn!("Could not deserialize JSON via TestConfigFromSerde: {err}");
                                err
                            })
                            .ok()
                    })
                    .map(Self::from)
            })
    }

    fn workspace_root() -> Option<PathBuf> {
        env::var("WORKSPACE_ROOT").ok().map(PathBuf::from)
    }

    pub fn external_test_executor_path(&self) -> Option<String> {
        const QUERY_COMPILER_TEST_EXECUTOR: &str = "libs/driver-adapters/executor/script/testd-qc.sh";

        Self::workspace_root()
            .or_else(|| {
                exit_with_message(
                    "WORKSPACE_ROOT needs to be correctly set to the root of the prisma-engines repository",
                )
            })
            .map(|path| path.join(QUERY_COMPILER_TEST_EXECUTOR))
            .and_then(|path| path.to_str().map(|s| s.to_owned()))
    }

    fn validate(&self) {
        if let Some(file) = self.external_test_executor_path().as_ref() {
            let path = PathBuf::from(file);
            let md = path.metadata();
            if !path.exists() || md.is_err() || !md.unwrap().is_file() {
                exit_with_message(&format!("The external test executor path `{file}` must be a file"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let is_executable = match path.metadata() {
                    Err(_) => false,
                    Ok(md) => md.permissions().mode() & 0o111 != 0,
                };
                if !is_executable {
                    exit_with_message(&format!(
                        "The external test executor file `{file}` must be have permissions to execute"
                    ));
                }
            }
        }
    }

    pub fn connector(&self) -> &str {
        self.connector.as_str()
    }

    pub fn connector_version(&self) -> Option<&str> {
        self.connector_version.as_deref()
    }

    pub fn test_connector(&self) -> TestResult<(ConnectorTag, ConnectorVersion)> {
        let version = self.parse_connector_version()?;
        let tag = match version {
            ConnectorVersion::SqlServer(_) => &SqlServerConnectorTag as ConnectorTag,
            ConnectorVersion::Postgres(_) => &PostgresConnectorTag,
            ConnectorVersion::MySql(_) => &MySqlConnectorTag,
            ConnectorVersion::MongoDb(_) => &MongoDbConnectorTag,
            ConnectorVersion::Sqlite(_) => &SqliteConnectorTag,
            ConnectorVersion::CockroachDb(_) => &CockroachDbConnectorTag,
            ConnectorVersion::Vitess(_) => &VitessConnectorTag,
        };

        Ok((tag, version))
    }

    pub fn max_bind_values(&self) -> Option<usize> {
        let version = self.parse_connector_version().unwrap();
        let local_mbv = self.with_driver_adapter().max_bind_values;

        local_mbv.or_else(|| version.max_bind_values())
    }

    fn parse_connector_version(&self) -> TestResult<ConnectorVersion> {
        ConnectorVersion::try_from((self.connector(), self.connector_version()))
    }

    #[rustfmt::skip]
    pub fn for_external_executor(&self) -> Vec<(String, String)> {
        let with_driver_adapter = self.with_driver_adapter();

        vec!(
            (
                "CONNECTOR".to_string(),
                self.connector().to_string()
            ),
            (
                "CONNECTOR_VERSION".to_string(),
                self.connector_version().unwrap_or("").to_string()
            ),
            (
                "DRIVER_ADAPTER".to_string(),
                with_driver_adapter.adapter.to_string()
            ),
            (
                "DRIVER_ADAPTER_CONFIG".to_string(),
                with_driver_adapter.json_stringify_config(),
            ),
            (
                "PRISMA_DISABLE_QUAINT_EXECUTORS".to_string(),
                "1".to_string(),
            ),
        )
    }
}

fn config_path(mut path: PathBuf) -> PathBuf {
    path.push(TEST_CONFIG_FILE_NAME);
    path
}
