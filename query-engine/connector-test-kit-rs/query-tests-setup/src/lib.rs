mod config;
mod connector_tag;
mod datamodel_rendering;
mod error;
mod ignore_lists;
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
pub use request_handlers::{GraphqlBody, MultiQuery};
pub use runner::*;
pub use schema_gen::*;
pub use templating::*;

use colored::Colorize;
use futures::{future::Either, FutureExt};
use prisma_metrics::{MetricRecorder, MetricRegistry, WithMetricsInstrumentation};
use psl::datamodel_connector::ConnectorCapabilities;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use tokio::runtime::Builder;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing_futures::WithSubscriber;

pub type TestResult<T> = Result<T, TestError>;

/// Test configuration, loaded once at runtime.
pub static CONFIG: LazyLock<TestConfig> = LazyLock::new(TestConfig::load);

/// The log level from the environment.
pub static ENV_LOG_LEVEL: LazyLock<String> =
    LazyLock::new(|| std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_owned()));

/// Engine protocol used to run tests. Either 'graphql' or 'json'.
pub static ENGINE_PROTOCOL: LazyLock<String> =
    LazyLock::new(|| std::env::var("PRISMA_ENGINE_PROTOCOL").unwrap_or_else(|_| "graphql".to_owned()));

/// Teardown of a test setup.
async fn teardown_project(datamodel: &str, db_schemas: &[&str], schema_id: Option<usize>) -> TestResult<()> {
    if let Some(schema_id) = schema_id {
        let params = serde_json::json!({ "schemaId": schema_id });
        executor_process_request::<serde_json::Value>("teardown", params).await?;
    }

    Ok(qe_setup::teardown(datamodel, db_schemas).await?)
}

/// Helper method to allow a sync shell function to run the async test blocks.
fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

pub fn setup_metrics() -> (MetricRegistry, MetricRecorder) {
    let metrics = MetricRegistry::new();
    let recorder = MetricRecorder::new(metrics.clone()).with_initialized_prisma_metrics();
    (metrics, recorder)
}

/// Taken from Reddit. Enables taking an async function pointer which takes references as param
/// https://www.reddit.com/r/rust/comments/jvqorj/hrtb_with_async_functions/
pub trait AsyncFn<'a, A: 'a, B: 'a, T>: Copy + 'static {
    type Fut: Future<Output = T> + 'a;

    fn call(self, a: &'a A, b: &'a B) -> Self::Fut;
}

impl<'a, A, B, Fut, F> AsyncFn<'a, A, B, Fut::Output> for F
where
    A: 'a,
    B: 'a,
    Fut: Future + 'a,
    F: Fn(&'a A, &'a B) -> Fut + Copy + 'static,
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
    required_capabilities: ConnectorCapabilities,
    (suite_name, test_name): (&str, &str),
    test_fn: F,
    test_function_name: &'static str,
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
        std::any::type_name::<F>(),
        test_function_name,
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
    required_capabilities: ConnectorCapabilities,
    (suite_name, test_name): (&str, &str),
    test_fn: &dyn for<'a> Fn(&'a Runner, &'a DatamodelWithParams) -> BoxFuture<'a, TestResult<()>>,
    test_fn_full_name: &'static str,
    original_test_function_name: &'static str,
) {
    let full_test_name = build_full_test_name(test_fn_full_name, original_test_function_name);

    if ignore_lists::is_ignored(&full_test_name) {
        return;
    }

    let expected_to_fail = ignore_lists::is_expected_to_fail(&full_test_name);
    let failed = &AtomicBool::new(false);

    static RELATION_TEST_IDX: LazyLock<Option<usize>> =
        LazyLock::new(|| std::env::var("RELATION_TEST_IDX").ok().and_then(|s| s.parse().ok()));

    let (dms, capabilities) = schema_with_relation(on_parent, on_child, id_only);

    insta::allow_duplicates! {
        for (i, (dm, caps)) in dms.into_iter().zip(capabilities.into_iter()).enumerate() {
            if RELATION_TEST_IDX.map(|idx| idx != i).unwrap_or(false) {
                continue;
            }

            let required_capabilities_for_test = required_capabilities | caps;
            let test_db_name = format!("{suite_name}_{test_name}_{i}");
            let template = dm.datamodel().to_owned();
            let (connector, version) = CONFIG.test_connector().unwrap();

            if !should_run(&connector, &version, only, exclude, required_capabilities_for_test) {
                continue;
            }

            let datamodel = render_test_datamodel(&test_db_name, template, &[], None, Default::default(), Default::default(), None);
            let (connector_tag, version) = CONFIG.test_connector().unwrap();
            let (metrics, recorder) = setup_metrics();
            let (log_capture, log_tx) = TestLogCapture::new();

            run_with_tokio(
                async move {
                    println!("Used datamodel:\n {}", datamodel.yellow());
                    let override_local_max_bind_values = None;
                    let runner = Runner::load(datamodel.clone(), &[], version, connector_tag, override_local_max_bind_values, metrics, log_capture)
                        .await
                        .unwrap();

                    let test_future = if expected_to_fail {
                        Either::Left(async {
                            match AssertUnwindSafe(test_fn(&runner, &dm)).catch_unwind().await {
                                Ok(Ok(_)) => {},
                                Ok(Err(err)) => {
                                    failed.store(true, Ordering::Relaxed);
                                    eprintln!("test failed as expected: {err}");
                                }
                                Err(panic) => {
                                    failed.store(true, Ordering::Relaxed);
                                    eprintln!(
                                        "test panicked as expected: {}",
                                        panic_utils::downcast_box_to_string(panic).unwrap_or_default()
                                    );
                                }
                            };
                            Ok(())
                        })
                    } else {
                        Either::Right(test_fn(&runner, &dm))
                    };

                    test_future.with_subscriber(test_tracing_subscriber(
                        ENV_LOG_LEVEL.to_string(),
                        log_tx,
                    )).with_recorder(recorder)
                    .await.unwrap();

                    if let Err(e) = teardown_project(&datamodel, Default::default(), runner.schema_id()).await {
                        if expected_to_fail {
                            eprintln!("Teardown failed: {e}");
                        } else {
                            panic!("Teardown failed: {e}");
                        }
                    }

                }
            );

            if failed.load(Ordering::Relaxed) {
                break;
            }
        }
    }

    if expected_to_fail && !failed.load(Ordering::Relaxed) {
        panic!("expected at least one of the variants of the relation test to fail but they all succeeded");
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
    capabilities: ConnectorCapabilities,
    excluded_features: &[&str],
    excluded_executors: &[&str],
    handler: fn() -> String,
    db_schemas: &[&str],
    db_extensions: &[&str],
    referential_override: Option<String>,
    test_fn: T,
    test_function_name: &'static str,
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
        excluded_executors,
        handler,
        db_schemas,
        db_extensions,
        referential_override,
        &boxify(test_fn),
        std::any::type_name::<T>(),
        test_function_name,
    )
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn run_connector_test_impl(
    test_database_name: &str,
    only: &[(&str, Option<&str>)],
    exclude: &[(&str, Option<&str>)],
    capabilities: ConnectorCapabilities,
    excluded_features: &[&str],
    excluded_executors: &[&str],
    handler: fn() -> String,
    db_schemas: &[&str],
    db_extensions: &[&str],
    referential_override: Option<String>,
    test_fn: &dyn Fn(Runner) -> BoxFuture<'static, TestResult<()>>,
    test_fn_full_name: &'static str,
    original_test_function_name: &'static str,
) {
    if CONFIG.with_driver_adapter().is_some_and(|da| {
        excluded_executors
            .iter()
            .any(|exec| exec.parse::<TestExecutor>() == Ok(da.test_executor))
    }) {
        return;
    }

    let (connector, version) = CONFIG.test_connector().unwrap();

    let full_test_name = build_full_test_name(test_fn_full_name, original_test_function_name);

    if ignore_lists::is_ignored(&full_test_name) {
        return;
    }

    if !should_run(&connector, &version, only, exclude, capabilities) {
        return;
    }

    let template = handler();
    let datamodel = crate::render_test_datamodel(
        test_database_name,
        template,
        excluded_features,
        referential_override,
        db_schemas,
        db_extensions,
        None,
    );
    let (connector_tag, version) = CONFIG.test_connector().unwrap();
    let (metrics, recorder) = crate::setup_metrics();

    let (log_capture, log_tx) = TestLogCapture::new();

    crate::run_with_tokio(async {
        println!("Used datamodel:\n {}", datamodel.yellow());
        let override_local_max_bind_values = None;
        let runner = Runner::load(
            datamodel.clone(),
            db_schemas,
            version,
            connector_tag,
            override_local_max_bind_values,
            metrics,
            log_capture,
        )
        .await
        .unwrap();
        let schema_id = runner.schema_id();

        let expected_to_fail = ignore_lists::is_expected_to_fail(&full_test_name);

        let test_future = if expected_to_fail {
            Either::Left(async {
                match AssertUnwindSafe(test_fn(runner)).catch_unwind().await {
                    Ok(Ok(_)) => panic!("expected this test to fail but it succeeded"),
                    Ok(Err(err)) => {
                        eprintln!("test failed as expected: {err}");
                        Ok(())
                    }
                    Err(panic) => {
                        eprintln!(
                            "test panicked as expected: {}",
                            panic_utils::downcast_box_to_string(panic).unwrap_or_default()
                        );
                        Ok(())
                    }
                }
            })
        } else {
            Either::Right(test_fn(runner))
        };

        if let Err(err) = test_future
            .with_subscriber(test_tracing_subscriber(ENV_LOG_LEVEL.to_string(), log_tx))
            .with_recorder(recorder)
            .await
        {
            // Print any traceback directly to stdout, so it remains readable
            eprintln!("Test failed due to an error:");
            eprintln!("=====");
            eprintln!("{err}");
            eprintln!("=====");
            panic!("💥 Test failed due to an error (see above)");
        }

        if let Err(e) = crate::teardown_project(&datamodel, db_schemas, schema_id).await {
            if expected_to_fail {
                eprintln!("Teardown failed: {e}");
            } else {
                panic!("Teardown failed: {e}");
            }
        }
    });
}

fn build_full_test_name(test_fn_full_name: &'static str, original_test_function_name: &'static str) -> String {
    let mut parts = test_fn_full_name.split("::").skip(1).collect::<Vec<_>>();
    parts.pop();
    parts.push(original_test_function_name);
    parts.join("::")
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

    pub async fn clear_logs(&mut self) {
        while self.rx.try_recv().is_ok() {}
    }
}
