use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace::Config, Resource},
    KeyValue,
};
use opentelemetry_otlp::Uninstall;
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
    telemetry_endpoint: Option<&'a str>,
}

impl<'a> Logger<'a> {
    /// Initialize a new global logger installer.
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            log_format: LogFormat::Json,
            enable_telemetry: false,
            telemetry_endpoint: None,
        }
    }

    /// Sets the STDOUT log output format. Default: Json.
    pub fn log_format(&mut self, log_format: LogFormat) {
        self.log_format = log_format;
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
    pub fn install(self) -> LoggerResult<Option<Uninstall>> {
        let filter = EnvFilter::from_default_env()
            .add_directive("tide=error".parse().unwrap())
            .add_directive("tonic=error".parse().unwrap())
            .add_directive("h2=error".parse().unwrap())
            .add_directive("hyper=error".parse().unwrap())
            .add_directive("tower=error".parse().unwrap());

        match self.log_format {
            LogFormat::Text => {
                if self.enable_telemetry {
                    let subscriber = FmtSubscriber::builder()
                        .with_env_filter(filter.add_directive("trace".parse().unwrap()))
                        .finish();

                    self.finalize(subscriber)
                } else {
                    let subscriber = FmtSubscriber::builder().with_max_level(tracing::Level::TRACE).finish();
                    self.finalize(subscriber)
                }
            }
            LogFormat::Json => {
                let subscriber = FmtSubscriber::builder().json().with_env_filter(filter).finish();
                self.finalize(subscriber)
            }
        }
    }

    fn finalize<T>(self, subscriber: T) -> LoggerResult<Option<Uninstall>>
    where
        T: SubscriberExt + Send + Sync + 'static + for<'span> LookupSpan<'span>,
    {
        if self.enable_telemetry {
            global::set_text_map_propagator(TraceContextPropagator::new());

            // A special parameter for Jaeger to set the service name in spans.
            let resource = Resource::new(vec![KeyValue::new("service.name", self.service_name)]);
            let config = Config::default().with_resource(resource);

            let mut builder = opentelemetry_otlp::new_pipeline().with_trace_config(config);

            if let Some(endpoint) = self.telemetry_endpoint {
                builder = builder.with_endpoint(endpoint);
            }

            let (tracer, guard) = builder.install().unwrap();

            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
            subscriber::set_global_default(subscriber.with(telemetry))?;

            Ok(Some(guard))
        } else {
            subscriber::set_global_default(subscriber)?;

            Ok(None)
        }
    }
}
