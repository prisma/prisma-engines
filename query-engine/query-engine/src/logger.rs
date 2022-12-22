use opentelemetry::{
    sdk::{
        trace::{Config, Tracer},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use query_core::is_user_facing_trace_filter;
use query_engine_metrics::MetricRegistry;
use tracing::{dispatcher::SetGlobalDefaultError, subscriber};
use tracing_subscriber::{filter::filter_fn, layer::SubscriberExt, EnvFilter, Layer};

use crate::{telemetry_capturing, LogFormat};

type LoggerResult<T> = Result<T, SetGlobalDefaultError>;

/// An installer for a global logger.
#[derive(Debug, Clone)]
pub struct Logger<'a> {
    service_name: &'static str,
    log_format: LogFormat,
    enable_telemetry: bool,
    log_queries: bool,
    telemetry_endpoint: Option<&'a str>,
    metrics: Option<MetricRegistry>,
    trace_capturer: Option<telemetry_capturing::traces::Exporter>,
}

impl<'a> Logger<'a> {
    /// Initialize a new global logger installer.
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            log_format: LogFormat::Json,
            enable_telemetry: false,
            log_queries: false,
            telemetry_endpoint: None,
            metrics: None,
            trace_capturer: None,
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

    /// Enables Jaeger telemetry.
    pub fn enable_telemetry(&mut self, enable_telemetry: bool) {
        self.enable_telemetry = enable_telemetry;
    }

    /// Sets a custom telemetry endpoint
    pub fn telemetry_endpoint(&mut self, endpoint: &'a str) {
        if endpoint.is_empty() {
            self.telemetry_endpoint = None
        } else {
            self.telemetry_endpoint = Some(endpoint);
        }
    }

    pub fn enable_metrics(&mut self, metrics: MetricRegistry) {
        self.metrics = Some(metrics);
    }

    pub fn enable_trace_capturer(&mut self, capture_logs: bool) -> Option<telemetry_capturing::traces::Exporter> {
        let capturer = if capture_logs {
            Some(telemetry_capturing::traces::Exporter::new())
        } else {
            None
        };
        self.trace_capturer = capturer.clone();
        capturer
    }

    /// Install logger as a global. Can be called only once per application
    /// instance. The returned guard value needs to stay in scope for the whole
    /// lifetime of the service.
    pub fn install(self) -> LoggerResult<()> {
        let filter = create_env_filter(self.log_queries);

        let telemetry = if self.enable_telemetry {
            let tracer = create_otel_tracer(self.service_name, self.telemetry_endpoint);
            let mut telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            // todo: This is replacing the telemetry tracer used by the otel tracer
            if let Some(exporter) = self.trace_capturer {
                let tracer = crate::telemetry_capturing::traces::setup_and_install_tracer_globally(exporter);
                telemetry = telemetry.with_tracer(tracer);
            }

            let is_user_trace = filter_fn(is_user_facing_trace_filter);
            let telemetry = telemetry.with_filter(is_user_trace);
            Some(telemetry)
        } else {
            None
        };

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
            .with(self.metrics)
            .with(telemetry);

        subscriber::set_global_default(subscriber)?;
        Ok(())
    }
}

fn create_otel_tracer(service_name: &'static str, collector_endpoint: Option<&str>) -> Tracer {
    if let Some(endpoint) = collector_endpoint {
        // A special parameter for Jaeger to set the service name in spans.
        let resource = Resource::new(vec![KeyValue::new("service.name", service_name)]);
        let config = Config::default().with_resource(resource);

        let builder = opentelemetry_otlp::new_pipeline().tracing().with_trace_config(config);
        let exporter = opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint);
        builder.with_exporter(exporter).install_simple().unwrap()
    } else {
        crate::tracer::new_pipeline().install_simple()
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
