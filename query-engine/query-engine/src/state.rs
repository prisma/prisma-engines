use crate::{context::PrismaContext, logger::Logger, opt::PrismaOpt, PrismaResult};
use psl::PreviewFeature;
use query_core::schema::QuerySchemaRef;
use query_engine_metrics::{setup as metric_setup, MetricRegistry};
use std::sync::Arc;
use tracing::Instrument;

//// Shared application state.
pub struct State {
    pub cx: Arc<PrismaContext>,
    pub enable_playground: bool,
    pub enable_debug_mode: bool,
    pub enable_metrics: bool,
}

impl State {
    /// Create a new instance of `State`.
    fn new(cx: PrismaContext, enable_playground: bool, enable_debug_mode: bool, enable_metrics: bool) -> Self {
        Self {
            cx: Arc::new(cx),
            enable_playground,
            enable_debug_mode,
            enable_metrics,
        }
    }

    pub fn get_metrics(&self) -> MetricRegistry {
        self.cx.metrics.clone()
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        self.cx.query_schema()
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            cx: self.cx.clone(),
            enable_playground: self.enable_playground,
            enable_debug_mode: self.enable_debug_mode,
            enable_metrics: self.enable_metrics,
        }
    }
}

pub async fn setup(opts: &PrismaOpt, install_logger: bool, metrics: Option<MetricRegistry>) -> PrismaResult<State> {
    let metrics = if metrics.is_none() {
        MetricRegistry::new()
    } else {
        metrics.unwrap()
    };

    let mut logger = Logger::new("prisma-engine-http");
    logger.log_format(opts.log_format());
    logger.log_queries(opts.log_queries());
    logger.enable_telemetry(opts.enable_open_telemetry);
    logger.telemetry_endpoint(&opts.open_telemetry_endpoint);
    logger.enable_metrics(metrics.clone());

    let trace_capturer = logger.enable_trace_capturer(opts.enable_logs_in_response);

    if install_logger {
        logger.install().unwrap();
    }

    if opts.enable_metrics || opts.dataproxy_metric_override {
        metric_setup();
    }

    let datamodel = opts.schema(false)?;
    let config = &datamodel.configuration;
    config.validate_that_one_datasource_is_provided()?;

    let enable_metrics = config.preview_features().contains(PreviewFeature::Metrics) || opts.dataproxy_metric_override;
    let span = tracing::info_span!("prisma:engine:connect");

    let cx = PrismaContext::builder(datamodel) //  opts.enable_raw_queries, metrics, logs_capture)
        .set_metrics(metrics)
        .set_trace_capturer(trace_capturer)
        .enable_raw_queries(opts.enable_raw_queries)
        .build()
        .instrument(span)
        .await?;

    let state = State::new(cx, opts.enable_playground, opts.enable_debug_mode, enable_metrics);
    Ok(state)
}
