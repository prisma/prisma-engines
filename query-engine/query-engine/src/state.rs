use crate::{context::PrismaContext, logger::Logger, opt::PrismaOpt, PrismaResult};
use psl::PreviewFeature;
use query_engine_metrics::{setup as metric_setup, MetricRegistry};
use std::sync::Arc;
use tracing::Instrument;

pub async fn setup(
    opts: &PrismaOpt,
    install_logger: bool,
    metrics: Option<MetricRegistry>,
) -> PrismaResult<Arc<PrismaContext>> {
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
        .set_engine_flags(opts.enable_playground, opts.enable_debug_mode, enable_metrics)
        .build()
        .instrument(span)
        .await?;

    let state = Arc::new(cx);
    Ok(state)
}
