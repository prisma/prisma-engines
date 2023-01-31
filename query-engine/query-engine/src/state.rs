use crate::{context::PrismaContext, logger::Logger, opt::PrismaOpt, PrismaResult};
use psl::PreviewFeature;
use query_core::{protocol::EngineProtocol, schema::QuerySchemaRef};
use query_engine_metrics::{setup as metric_setup, MetricRegistry};
use std::sync::Arc;
use tracing::Instrument;

//// Shared application state.
#[derive(Clone)]
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

    pub fn engine_protocol(&self) -> &EngineProtocol {
        self.cx.engine_protocol()
    }
}

pub async fn setup(opts: &PrismaOpt, install_logger: bool, metrics: Option<MetricRegistry>) -> PrismaResult<State> {
    let metrics = metrics.unwrap_or_default();

    let mut logger = Logger::new("prisma-engine-http");
    logger.log_format(opts.log_format());
    logger.log_queries(opts.log_queries());
    logger.enable_metrics(metrics.clone());
    logger.setup_telemetry(
        opts.enable_open_telemetry,
        opts.enable_telemetry_in_response,
        &opts.open_telemetry_endpoint,
    );

    if install_logger {
        logger.install().unwrap();
    }

    if opts.enable_metrics || opts.dataproxy_metric_override {
        metric_setup();
    }

    let datamodel = opts.schema(false)?;
    let config = &datamodel.configuration;
    let protocol = opts.engine_protocol(config.preview_features());
    config.validate_that_one_datasource_is_provided()?;

    let enable_metrics = config.preview_features().contains(PreviewFeature::Metrics) || opts.dataproxy_metric_override;

    let span = tracing::info_span!("prisma:engine:connect");

    let cx = PrismaContext::builder(datamodel, protocol)
        .set_metrics(metrics)
        .enable_raw_queries(opts.enable_raw_queries)
        .build()
        .instrument(span)
        .await?;

    let state = State::new(cx, opts.enable_playground, opts.enable_debug_mode, enable_metrics);
    Ok(state)
}
