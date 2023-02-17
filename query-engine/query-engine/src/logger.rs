use opentelemetry::{
    sdk::{trace::Config, Resource},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use query_core::telemetry;
use query_engine_metrics::MetricRegistry;
use tracing::{dispatcher::SetGlobalDefaultError, subscriber};
use tracing_subscriber::{filter::filter_fn, layer::SubscriberExt, Layer};

use crate::{opt::PrismaOpt, LogFormat};

type LoggerResult<T> = Result<T, SetGlobalDefaultError>;

/// An installer for a global logger.
#[derive(Debug, Clone)]
pub struct Logger {
    service_name: &'static str,
    log_format: LogFormat,
    log_queries: bool,
    tracing_config: TracingConfig,
    metrics: Option<MetricRegistry>,
}

// TracingConfig specifies how tracing will be exposed by the logger facility
#[derive(Debug, Clone)]
enum TracingConfig {
    // exposed means tracing will be exposed through an HTTP endpoint in a jaeger-compatible format
    Http(String),
    // captured means that traces will be captured in memory and exposed in the graphql response
    // logs will be also exposed in the response when capturing is enabled
    Captured,
    // stdout means that traces will be printed to standard output
    Stdout,
    // disabled means that tracing will be disabled
    Disabled,
}

impl Logger {
    /// Initialize a new global logger installer.
    pub fn new(service_name: &'static str, metrics: Option<MetricRegistry>, opts: &PrismaOpt) -> Self {
        let enable_telemetry = opts.enable_open_telemetry;
        let enable_capturing = opts.enable_telemetry_in_response;
        let endpoint = if opts.open_telemetry_endpoint.is_empty() {
            None
        } else {
            Some(opts.open_telemetry_endpoint.to_owned())
        };

        let tracing_config = match (enable_telemetry, enable_capturing, endpoint) {
            (_, true, _) => TracingConfig::Captured,
            (true, _, Some(endpoint)) => TracingConfig::Http(endpoint),
            (true, _, None) => TracingConfig::Stdout,
            _ => TracingConfig::Disabled,
        };

        Self {
            service_name,
            log_format: opts.log_format(),
            log_queries: opts.log_queries(),
            metrics,
            tracing_config,
        }
    }

    pub fn enable_metrics(&mut self, metrics: MetricRegistry) {
        self.metrics = Some(metrics);
    }

    pub fn is_metrics_enabled(&self) -> bool {
        self.metrics.is_some()
    }

    /// Install logger as a global. Can be called only once per application
    /// instance. The returned guard value needs to stay in scope for the whole
    /// lifetime of the service.
    pub fn install(&self) -> LoggerResult<()> {
        let filter = telemetry::helpers::env_filter(self.log_queries, telemetry::helpers::QueryEngineLogLevel::FromEnv);
        let is_user_trace = filter_fn(telemetry::helpers::user_facing_span_only_filter);

        let fmt_layer = match self.log_format {
            LogFormat::Text => {
                let fmt_layer = tracing_subscriber::fmt::layer().with_filter(filter);
                fmt_layer.boxed()
            }
            LogFormat::Json => {
                let fmt_layer = tracing_subscriber::fmt::layer().json().with_filter(filter);
                fmt_layer.boxed()
            }
        };

        let subscriber = tracing_subscriber::registry()
            .with(fmt_layer)
            .with(self.metrics.clone());

        match self.tracing_config {
            TracingConfig::Captured => {
                let log_queries = self.log_queries;
                telemetry::capturing::install_capturing_layer(subscriber, log_queries)
            }
            TracingConfig::Http(ref endpoint) => {
                // Opentelemetry is enabled, but capturing is disabled, there's an endpoint to export
                // the traces to.
                let resource = Resource::new(vec![KeyValue::new("service.name", self.service_name)]);
                let config = Config::default().with_resource(resource);
                let builder = opentelemetry_otlp::new_pipeline().tracing().with_trace_config(config);
                let exporter = opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint);
                let tracer = builder.with_exporter(exporter).install_simple().unwrap();
                let telemetry_layer = tracing_opentelemetry::layer()
                    .with_tracer(tracer)
                    .with_filter(is_user_trace);
                let subscriber = subscriber.with(telemetry_layer);
                subscriber::set_global_default(subscriber)?;
            }
            TracingConfig::Stdout => {
                // Opentelemetry is enabled, but capturing is disabled, and there's no endpoint to
                // export traces too. We export it to stdout
                let exporter = crate::tracer::ClientSpanExporter::default();
                let tracer = crate::tracer::install(Some(exporter), None);
                let telemetry_layer = tracing_opentelemetry::layer()
                    .with_tracer(tracer)
                    .with_filter(is_user_trace);
                let subscriber = subscriber.with(telemetry_layer);
                subscriber::set_global_default(subscriber)?;
            }
            TracingConfig::Disabled => {
                subscriber::set_global_default(subscriber)?;
            }
        }

        Ok(())
    }
}
