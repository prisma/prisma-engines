use opentelemetry::{
    sdk::{trace::Config, Resource},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use query_core::is_user_facing_trace_filter;
use query_engine_metrics::MetricRegistry;
use tracing::{dispatcher::SetGlobalDefaultError, subscriber};
use tracing_subscriber::{filter::filter_fn, layer::SubscriberExt, EnvFilter, Layer};

use crate::LogFormat;

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
    Exposed(String),
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
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            log_format: LogFormat::Json,
            log_queries: false,
            metrics: None,
            tracing_config: TracingConfig::Disabled,
        }
    }

    /// Sets the STDOUT log output format. Default: Json.
    pub fn log_format(&mut self, log_format: LogFormat) {
        self.log_format = log_format;
    }

    /// Enable query logging. Default: false.
    pub fn log_queries(&mut self, log_queries: bool) {
        self.log_queries = log_queries;
    }

    pub fn enable_metrics(&mut self, metrics: MetricRegistry) {
        self.metrics = Some(metrics);
    }

    pub fn setup_telemetry(&mut self, enable_telemetry: bool, enable_capturing: bool, endpoint: &str) {
        let endpoint = if endpoint.is_empty() {
            None
        } else {
            Some(endpoint.to_owned())
        };

        self.tracing_config = match (enable_telemetry, enable_capturing, endpoint) {
            (_, true, _) => TracingConfig::Captured,
            (true, _, Some(endpoint)) => TracingConfig::Exposed(endpoint),
            (true, _, None) => TracingConfig::Stdout,
            _ => TracingConfig::Disabled,
        };
    }

    pub fn is_metrics_enabled(&self) -> bool {
        self.metrics.is_some()
    }

    /// Install logger as a global. Can be called only once per application
    /// instance. The returned guard value needs to stay in scope for the whole
    /// lifetime of the service.
    pub fn install(&self) -> LoggerResult<()> {
        let filter = create_env_filter(self.log_queries);
        let is_user_trace = filter_fn(is_user_facing_trace_filter);

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
                // Capturing is enabled, it overrides otel exporting.
                let tracer = crate::telemetry_capturing::tracer().to_owned();
                let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                //.with_filter(is_user_trace);
                let subscriber = subscriber.with(telemetry_layer);
                subscriber::set_global_default(subscriber)?;
            }
            TracingConfig::Exposed(ref endpoint) => {
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
                let tracer = crate::tracer::new_pipeline()
                    .with_client_span_exporter()
                    .install_simple();
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

fn create_env_filter(log_queries: bool) -> EnvFilter {
    let mut filter = EnvFilter::from_default_env()
        .add_directive("tide=error".parse().unwrap())
        .add_directive("tonic=error".parse().unwrap())
        .add_directive("h2=error".parse().unwrap())
        .add_directive("hyper=error".parse().unwrap())
        .add_directive("tower=error".parse().unwrap());

    if let Ok(qe_log_level) = std::env::var("QE_LOG_LEVEL") {
        filter = filter
            .add_directive(format!("query_engine={}", &qe_log_level).parse().unwrap())
            .add_directive(format!("query_core={}", &qe_log_level).parse().unwrap())
            .add_directive(format!("query_connector={}", &qe_log_level).parse().unwrap())
            .add_directive(format!("sql_query_connector={}", &qe_log_level).parse().unwrap())
            .add_directive(format!("mongodb_query_connector={}", &qe_log_level).parse().unwrap());
    }

    if log_queries {
        // even when mongo queries are logged in debug mode, we want to log them if the log level is higher
        filter = filter
            .add_directive("quaint[{is_query}]=trace".parse().unwrap())
            .add_directive("mongodb_query_connector=debug".parse().unwrap());
    }

    filter
}
