use telemetry::{Exporter, filter};
use tracing::{dispatcher::SetGlobalDefaultError, subscriber};
use tracing_subscriber::{Layer, filter::FilterExt, layer::SubscriberExt};

use crate::{LogFormat, opt::PrismaOpt};

type LoggerResult<T> = Result<T, SetGlobalDefaultError>;

/// An installer for a global logger.
#[derive(Debug, Clone)]
pub(crate) struct Logger {
    log_format: LogFormat,
    log_queries: bool,
    tracing_config: TracingConfig,
    exporter: Exporter,
}

// TracingConfig specifies how tracing will be exposed by the logger facility
#[derive(Debug, Clone, Copy)]
pub(crate) enum TracingConfig {
    /// Logs and spans will be captured in memory and exposed in the response.
    LogsAndTracesInResponse,
    /// Logs will be printed to standard output, spans will be captured and
    /// exposed in the response.
    StdoutLogsAndTracesInResponse,
    // Logs will be printed to standard output, tracing is disabled.
    StdoutLogsOnly,
}

impl TracingConfig {
    pub fn should_capture(&self) -> bool {
        matches!(
            self,
            TracingConfig::LogsAndTracesInResponse | TracingConfig::StdoutLogsAndTracesInResponse
        )
    }
}

impl Logger {
    /// Initialize a new global logger installer.
    pub fn new(opts: &PrismaOpt) -> Self {
        let enable_telemetry = opts.enable_open_telemetry;
        let enable_capturing = opts.enable_telemetry_in_response;

        let tracing_config = match (enable_telemetry, enable_capturing) {
            (_, true) => TracingConfig::LogsAndTracesInResponse,
            (true, false) => TracingConfig::StdoutLogsAndTracesInResponse,
            (false, false) => TracingConfig::StdoutLogsOnly,
        };

        Self {
            log_format: opts.log_format(),
            log_queries: opts.log_queries(),
            tracing_config,
            exporter: Exporter::new(),
        }
    }

    /// Install logger as a global. Can be called only once per application
    /// instance.
    pub fn install(self) -> LoggerResult<Self> {
        let filter = filter::EnvFilterBuilder::new().log_queries(self.log_queries).build();

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

        let subscriber = tracing_subscriber::registry().with(fmt_layer);

        match self.tracing_config {
            TracingConfig::LogsAndTracesInResponse => {
                let subscriber = subscriber.with(
                    telemetry::layer(self.exporter.clone()).with_filter(
                        filter::user_facing_spans()
                            .or(filter::events().and(filter::EnvFilterBuilder::new().log_queries(true).build())),
                    ),
                );
                subscriber::set_global_default(subscriber)?;
            }
            TracingConfig::StdoutLogsAndTracesInResponse => {
                let subscriber =
                    subscriber.with(telemetry::layer(self.exporter.clone()).with_filter(filter::user_facing_spans()));
                subscriber::set_global_default(subscriber)?;
            }
            TracingConfig::StdoutLogsOnly => {
                subscriber::set_global_default(subscriber)?;
            }
        }

        Ok(self)
    }

    pub fn tracing_config(&self) -> TracingConfig {
        self.tracing_config
    }

    pub fn exporter(&self) -> &Exporter {
        &self.exporter
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            log_format: LogFormat::Text,
            log_queries: false,
            tracing_config: TracingConfig::StdoutLogsOnly,
            exporter: Exporter::new(),
        }
    }
}
