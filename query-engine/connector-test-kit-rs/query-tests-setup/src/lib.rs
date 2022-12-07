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
pub use query_result::*;
pub use request_handlers::{GraphQlBody, MultiQuery};
pub use runner::*;
pub use schema_gen::*;
pub use templating::*;

use colored::Colorize;
use lazy_static::lazy_static;
use psl::datamodel_connector::ConnectorCapability;
use query_engine_metrics::MetricRegistry;
use std::future::Future;
use std::sync::Once;
use tokio::runtime::Builder;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing_futures::WithSubscriber;

pub type TestResult<T> = Result<T, TestError>;

lazy_static! {
    /// Test configuration, loaded once at runtime.
    pub static ref CONFIG: TestConfig = TestConfig::load().unwrap();

    /// The log level from the environment.
    pub static ref ENV_LOG_LEVEL: String = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_owned());
}

/// Setup of everything as defined in the passed datamodel.
pub async fn setup_project(datamodel: &str, db_schemas: &[&str]) -> TestResult<()> {
    Ok(qe_setup::setup(datamodel, db_schemas).await?)
}

/// Teardown of a test setup.
pub async fn teardown_project(datamodel: &str, db_schemas: &[&str]) -> TestResult<()> {
    Ok(qe_setup::teardown(datamodel, db_schemas).await?)
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
pub trait AsyncFn<'a, A: 'a, B: 'a, T>: Copy + 'static {
    type Fut: Future<Output = T> + 'a;

    fn call(self, a: &'a A, b: &'a B) -> Self::Fut;
}

impl<'a, A: 'a, B: 'a, Fut: Future + 'a, F: Fn(&'a A, &'a B) -> Fut + Copy + 'static> AsyncFn<'a, A, B, Fut::Output>
    for F
{
    type Fut = Fut;

    fn call(self, a: &'a A, b: &'a B) -> Self::Fut {
        self(a, b)
    }
}

type BoxFuture<'a, O> = std::pin::Pin<Box<dyn std::future::Future<Output = O> + 'a>>;

#[allow(clippy::too_many_arguments)]
pub fn run_relation_link_test<F>(
    on_parent: &RelationField,
    on_child: &RelationField,
    id_only: bool,
    only: &[(&str, Option<&str>)],
    exclude: &[(&str, Option<&str>)],
    required_capabilities: &[ConnectorCapability],
    (suite_name, test_name): (&str, &str),
    test_fn: F,
) where
    F: (for<'a> AsyncFn<'a, Runner, DatamodelWithParams, TestResult<()>>) + 'static,
{
    // The implementation of the function is separated from this façade because of monomorphization
    // cost. Only `boxify` is instantiated for each instance of run_relation_link_test, not the
    // whole larger function body. This measurably improves compile times of query-engine-tests.
    /// Helper for test return type erasure.
    fn boxify<F>(f: F) -> impl for<'a> Fn(&'a Runner, &'a DatamodelWithParams) -> BoxFuture<'a, TestResult<()>>
    where
        F: (for<'a> AsyncFn<'a, Runner, DatamodelWithParams, TestResult<()>>) + 'static,
    {
        move |runner, datamodel| Box::pin(async move { f.call(runner, datamodel).await })
    }

    run_relation_link_test_impl(
        on_parent,
        on_child,
        id_only,
        only,
        exclude,
        required_capabilities,
        (suite_name, test_name),
        &boxify(test_fn),
    )
}

#[allow(clippy::too_many_arguments)]
#[inline(never)] // currently not inlined, but let's make sure it doesn't change
fn run_relation_link_test_impl(
    on_parent: &RelationField,
    on_child: &RelationField,
    id_only: bool,
    only: &[(&str, Option<&str>)],
    exclude: &[(&str, Option<&str>)],
    required_capabilities: &[ConnectorCapability],
    (suite_name, test_name): (&str, &str),
    test_fn: &dyn for<'a> Fn(&'a Runner, &'a DatamodelWithParams) -> BoxFuture<'a, TestResult<()>>,
) {
    let (dms, capabilities) = schema_with_relation(on_parent, on_child, id_only);
    let mut required_capabilities_for_test = Vec::with_capacity(required_capabilities.len());

    for (i, (dm, caps)) in dms.into_iter().zip(capabilities.into_iter()).enumerate() {
        required_capabilities_for_test.clear();
        required_capabilities_for_test.extend(required_capabilities.iter());
        required_capabilities_for_test.extend(caps);

        let test_db_name = format!("{suite_name}_{test_name}_{i}");
        let template = dm.datamodel().to_owned();

        if !ConnectorTag::should_run(only, exclude, &required_capabilities_for_test) {
            continue;
        }

        let datamodel = render_test_datamodel(&test_db_name, template, &[], None, Default::default(), None);
        let connector = CONFIG.test_connector_tag().unwrap();
        let metrics = setup_metrics();
        let metrics_for_subscriber = metrics.clone();
        let (log_capture, log_tx) = TestLogCapture::new();

        run_with_tokio(
            async move {
                println!("Used datamodel:\n {}", datamodel.clone().yellow());
                setup_project(&datamodel, Default::default()).await.unwrap();

                let runner = Runner::load(CONFIG.runner(), datamodel.clone(), connector, metrics, log_capture)
                    .await
                    .unwrap();

                test_fn(&runner, &dm).await.unwrap();

                teardown_project(&datamodel, Default::default()).await.unwrap();
            }
            .with_subscriber(test_tracing_subscriber(&ENV_LOG_LEVEL, metrics_for_subscriber, log_tx)),
        );
    }
}

pub trait ConnectorTestFn: Copy + 'static {
    type Fut: Future<Output = TestResult<()>>;

    fn call(self, runner: Runner) -> Self::Fut;
}

impl<T, F> ConnectorTestFn for T
where
    T: Fn(Runner) -> F + Copy + 'static,
    F: Future<Output = TestResult<()>>,
{
    type Fut = F;

    fn call(self, runner: Runner) -> Self::Fut {
        self(runner)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_connector_test<T>(
    test_database_name: &str,
    only: &[(&str, Option<&str>)],
    exclude: &[(&str, Option<&str>)],
    capabilities: &[ConnectorCapability],
    excluded_features: &[&str],
    handler: fn() -> String,
    db_schemas: &[&str],
    referential_override: Option<String>,
    test_fn: T,
) where
    T: ConnectorTestFn,
{
    // The implementation of the function is separated from this façade because of monomorphization
    // cost. Only `boxify` is instantiated for each instance of run_connector_test, not the whole
    // larger function body. This measurably improves compile times of query-engine-tests.
    fn boxify(test_fn: impl ConnectorTestFn) -> impl Fn(Runner) -> BoxFuture<'static, TestResult<()>> {
        move |runner| Box::pin(test_fn.call(runner))
    }

    run_connector_test_impl(
        test_database_name,
        only,
        exclude,
        capabilities,
        excluded_features,
        handler,
        db_schemas,
        referential_override,
        &boxify(test_fn),
    )
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn run_connector_test_impl(
    test_database_name: &str,
    only: &[(&str, Option<&str>)],
    exclude: &[(&str, Option<&str>)],
    capabilities: &[ConnectorCapability],
    excluded_features: &[&str],
    handler: fn() -> String,
    db_schemas: &[&str],
    referential_override: Option<String>,
    test_fn: &dyn Fn(Runner) -> BoxFuture<'static, TestResult<()>>,
) {
    if !ConnectorTag::should_run(only, exclude, capabilities) {
        return;
    }

    let template = handler();
    let datamodel = crate::render_test_datamodel(
        test_database_name,
        template,
        excluded_features,
        referential_override,
        db_schemas,
        None,
    );
    let connector = CONFIG.test_connector_tag().unwrap();
    let metrics = crate::setup_metrics();
    let metrics_for_subscriber = metrics.clone();

    let (log_capture, log_tx) = TestLogCapture::new();

    crate::run_with_tokio(
        async {
            println!("Used datamodel:\n {}", datamodel.clone().yellow());
            crate::setup_project(&datamodel, db_schemas).await.unwrap();

            let runner = Runner::load(
                crate::CONFIG.runner(),
                datamodel.clone(),
                connector,
                metrics,
                log_capture,
            )
            .await
            .unwrap();

            test_fn(runner).await.unwrap();

            crate::teardown_project(&datamodel, db_schemas).await.unwrap();
        }
        .with_subscriber(test_tracing_subscriber(&ENV_LOG_LEVEL, metrics_for_subscriber, log_tx)),
    );
}

pub type LogEmit = UnboundedSender<String>;
pub struct TestLogCapture {
    rx: UnboundedReceiver<String>,
}

impl TestLogCapture {
    pub fn new() -> (Self, LogEmit) {
        let (tx, rx) = unbounded_channel();
        (Self { rx }, tx)
    }

    pub async fn get_logs(&mut self) -> Vec<String> {
        let mut logs = Vec::new();
        while let Ok(log_line) = self.rx.try_recv() {
            logs.push(log_line)
        }

        logs
    }
}
