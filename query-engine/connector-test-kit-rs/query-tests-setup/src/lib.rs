#![allow(clippy::derive_partial_eq_without_eq)]

mod config;
mod connector_tag;
mod datamodel_rendering;
mod error;
mod logging;
mod query_result;
mod runner;
mod schema_gen;
mod templating;

pub use config::*;
pub use connector_tag::*;
pub use datamodel_rendering::*;
pub use error::*;
pub use logging::*;
pub use query_core;
use query_engine_metrics::MetricRegistry;
pub use query_result::*;
pub use runner::*;
pub use schema_gen::*;
pub use templating::*;

use colored::Colorize;
use datamodel_connector::ConnectorCapability;
use lazy_static::lazy_static;
use std::future::Future;
use std::sync::Once;
use tokio::runtime::Builder;
use tracing_futures::WithSubscriber;

pub type TestResult<T> = Result<T, TestError>;

lazy_static! {
    /// Test configuration, loaded once at runtime.
    pub static ref CONFIG: TestConfig = TestConfig::load().unwrap();
}

/// Setup of everything as defined in the passed datamodel.
pub async fn setup_project(datamodel: &str) -> TestResult<()> {
    Ok(qe_setup::setup(datamodel).await?)
}

/// Teardown of a test setup.
pub async fn teardown_project(datamodel: &str) -> TestResult<()> {
    Ok(qe_setup::teardown(datamodel).await?)
}

/// Helper method to allow a sync shell function to run the async test blocks.
pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

static METRIC_RECORDER: Once = Once::new();

pub fn setup_metrics() -> MetricRegistry {
    let metrics = MetricRegistry::new();
    METRIC_RECORDER.call_once(|| {
        query_engine_metrics::setup();
    });
    metrics
}

/// Taken from Reddit. Enables taking an async function pointer which takes references as param
/// https://www.reddit.com/r/rust/comments/jvqorj/hrtb_with_async_functions/
pub trait AsyncFn<'a, A: 'a, B: 'a, T> {
    type Fut: Future<Output = T> + 'a;

    fn call(self, a: &'a A, b: &'a B) -> Self::Fut;
}

impl<'a, A: 'a, B: 'a, Fut: Future + 'a, F: FnOnce(&'a A, &'a B) -> Fut> AsyncFn<'a, A, B, Fut::Output> for F {
    type Fut = Fut;

    fn call(self, a: &'a A, b: &'a B) -> Self::Fut {
        self(a, b)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_relation_link_test<F>(
    enabled_connectors: Vec<ConnectorTag>,
    capabilities: &mut Vec<ConnectorCapability>,
    required_capabilities: Vec<&str>,
    datamodel: &str,
    dm_with_params: &str,
    test_name: &str,
    test_database: &str,
    test_fn: F,
) where
    F: for<'a> AsyncFn<'a, Runner, DatamodelWithParams, TestResult<()>>,
{
    let config = &CONFIG;
    let mut required_capabilities = required_capabilities
        .into_iter()
        .map(|cap| cap.parse::<ConnectorCapability>().unwrap())
        .collect::<Vec<_>>();

    if !required_capabilities.is_empty() {
        capabilities.append(&mut required_capabilities);
    }

    let template = datamodel.to_string();
    let dm_with_params_json: DatamodelWithParams = dm_with_params.parse().unwrap();

    if ConnectorTag::should_run(config, &enabled_connectors, capabilities, test_name) {
        let datamodel = render_test_datamodel(config, test_database, template, &[], None);
        let connector = config.test_connector_tag().unwrap();
        let requires_teardown = connector.requires_teardown();
        let metrics = setup_metrics();
        let metrics_for_subscriber = metrics.clone();

        run_with_tokio(
            async move {
                tracing::debug!("Used datamodel:\n {}", datamodel.clone().yellow());
                setup_project(&datamodel).await.unwrap();

                let runner = Runner::load(config.runner(), datamodel.clone(), connector, metrics)
                    .await
                    .unwrap();

                test_fn.call(&runner, &dm_with_params_json).await.unwrap();

                if requires_teardown {
                    teardown_project(&datamodel).await.unwrap();
                }
            }
            .with_subscriber(test_tracing_subscriber(
                std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                metrics_for_subscriber,
            )),
        );
    }
}
