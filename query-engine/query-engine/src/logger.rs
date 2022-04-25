use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace::Config, Resource},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing::{dispatcher::SetGlobalDefaultError, subscriber};
use tracing_subscriber::{layer::SubscriberExt, registry::LookupSpan, EnvFilter, FmtSubscriber};

use crate::LogFormat;

type LoggerResult<T> = Result<T, SetGlobalDefaultError>;

/// An installer for a global logger.
#[derive(Debug, Clone)]
pub struct Logger<'a> {
    service_name: &'static str,
    log_format: LogFormat,
    enable_telemetry: bool,
    log_queries: bool,
    telemetry_endpoint: Option<&'a str>,
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

    /// Sets a custom telemetry endpoint (default: http://localhost:4317)
    pub fn telemetry_endpoint(&mut self, endpoint: &'a str) {
        self.telemetry_endpoint = Some(endpoint);
    }

    /// Install logger as a global. Can be called only once per application
    /// instance. The returned guard value needs to stay in scope for the whole
    /// lifetime of the service.
    pub fn install(self) -> LoggerResult<()> {
        let mut filter = EnvFilter::from_default_env()
            .add_directive("tide=error".parse().unwrap())
            .add_directive("tonic=error".parse().unwrap())
            .add_directive("h2=error".parse().unwrap())
            .add_directive("hyper=error".parse().unwrap())
            .add_directive("tower=error".parse().unwrap());

        if self.log_queries {
            filter = filter.add_directive("quaint[{is_query}]=trace".parse().unwrap());
        }

        match self.log_format {
            LogFormat::Text => {
                if self.enable_telemetry {
                    let subscriber = FmtSubscriber::builder()
                        .with_env_filter(filter.add_directive("trace".parse().unwrap()))
                        .finish();

                    self.finalize(subscriber)
                } else {
                    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
                    self.finalize(subscriber)
                }
            }
            LogFormat::Json => {
                let subscriber = FmtSubscriber::builder().json().with_env_filter(filter).finish();
                self.finalize(subscriber)
            }
        }
    }

    fn finalize<T>(self, subscriber: T) -> LoggerResult<()>
    where
        T: SubscriberExt + Send + Sync + 'static + for<'span> LookupSpan<'span>,
    {
        if self.enable_telemetry {
            global::set_text_map_propagator(TraceContextPropagator::new());

            // A special parameter for Jaeger to set the service name in spans.
            let resource = Resource::new(vec![KeyValue::new("service.name", self.service_name)]);
            let config = Config::default().with_resource(resource);

            let mut builder = opentelemetry_otlp::new_pipeline().tracing().with_trace_config(config);
            let mut exporter = opentelemetry_otlp::new_exporter().tonic();

            if let Some(endpoint) = self.telemetry_endpoint {
                exporter = exporter.with_endpoint(endpoint);
            }

            builder = builder.with_exporter(exporter);

            let tracer = builder.install_simple().unwrap();

            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            subscriber::set_global_default(subscriber.with(telemetry))?;

            Ok(())
        } else {
            subscriber::set_global_default(subscriber)?;

            Ok(())
        }
    }
}
