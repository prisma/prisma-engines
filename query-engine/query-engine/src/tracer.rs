use async_trait::async_trait;
use opentelemetry::{
    global,
    sdk::{self, export::ExportError},
    sdk::{
        export::trace::{ExportResult, SpanData, SpanExporter},
        propagation::TraceContextPropagator,
    },
    trace::TracerProvider,
};
use query_core::spans_to_json;
use std::io::{stdout, Stdout};
use std::{fmt::Debug, io::Write};

/// Pipeline builder
#[derive(Debug)]
pub struct PipelineBuilder {
    trace_config: Option<sdk::trace::Config>,
    exporter: Option<ClientSpanExporter>,
}

/// Create a new stdout exporter pipeline builder.
pub fn new_pipeline() -> PipelineBuilder {
    PipelineBuilder::default()
}

impl Default for PipelineBuilder {
    /// Return the default pipeline builder.
    fn default() -> Self {
        Self {
            trace_config: None,
            exporter: None,
        }
    }
}

impl PipelineBuilder {
    /// Assign the SDK trace configuration.
    pub fn with_trace_config(mut self, config: sdk::trace::Config) -> Self {
        self.trace_config = Some(config);
        self
    }

    /// Assign the SDK trace configuration.
    pub fn with_client_span_exporter(mut self) -> Self {
        self.exporter = Some(ClientSpanExporter::new());
        self
    }
}

impl PipelineBuilder {
    pub fn install_simple(mut self) -> sdk::trace::Tracer {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut provider_builder = sdk::trace::TracerProvider::builder();

        if let Some(exporter) = self.exporter {
            provider_builder = provider_builder.with_simple_exporter(exporter);
        }

        if let Some(config) = self.trace_config.take() {
            provider_builder = provider_builder.with_config(config);
        }
        let provider = provider_builder.build();
        let tracer = provider.tracer("opentelemetry");
        global::set_tracer_provider(provider);

        tracer
    }
}

/// A [`ClientSpanExporter`] that sends spans to stdout.
#[derive(Debug)]
pub struct ClientSpanExporter {
    writer: Stdout,
}

impl ClientSpanExporter {
    pub fn new() -> Self {
        Self { writer: stdout() }
    }
}

impl Default for ClientSpanExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SpanExporter for ClientSpanExporter {
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let result = spans_to_json(&batch);

        if let Err(err) = writeln!(self.writer, "{result}") {
            Err(ClientSpanExporterError(err).into())
        } else {
            Ok(())
        }
    }
}

// ClientSpanExporter exporter's error
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
struct ClientSpanExporterError(std::io::Error);

impl ExportError for ClientSpanExporterError {
    fn exporter_name(&self) -> &'static str {
        "stdout"
    }
}
