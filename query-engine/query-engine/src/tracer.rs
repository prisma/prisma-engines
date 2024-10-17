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

use std::io::{stdout, Stdout};
use std::{fmt::Debug, io::Write};

pub fn install<E>(exporter: Option<E>, mut tracer_config: Option<sdk::trace::Config>) -> sdk::trace::Tracer
where
    E: SpanExporter + 'static,
{
    global::set_text_map_propagator(TraceContextPropagator::new());
    let mut provider_builder = sdk::trace::TracerProvider::builder();

    if let Some(exporter) = exporter {
        provider_builder = provider_builder.with_simple_exporter(exporter);
    }

    if let Some(config) = tracer_config.take() {
        provider_builder = provider_builder.with_config(config);
    }
    let provider = provider_builder.build();
    let tracer = provider.tracer("opentelemetry");
    global::set_tracer_provider(provider);

    tracer
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
        let result = telemetry::helpers::spans_to_json(batch);

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
