use async_trait::async_trait;
use opentelemetry::{
    global,
    sdk::{
        self,
        export::trace::{ExportResult, SpanData, SpanExporter},
        propagation::TraceContextPropagator,
    },
    trace::{TraceId, TracerProvider},
};
use query_core::spans_to_json;
use std::fmt::Debug;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

/// Pipeline builder
#[derive(Debug)]
pub struct PipelineBuilder {
    trace_config: Option<sdk::trace::Config>,
}

/// Create a new stdout exporter pipeline builder.
pub fn new_pipeline() -> PipelineBuilder {
    PipelineBuilder::default()
}

impl Default for PipelineBuilder {
    /// Return the default pipeline builder.
    fn default() -> Self {
        Self { trace_config: None }
    }
}

impl PipelineBuilder {
    /// Assign the SDK trace configuration.
    pub fn with_trace_config(mut self, config: sdk::trace::Config) -> Self {
        self.trace_config = Some(config);
        self
    }
}

impl PipelineBuilder {
    pub fn install_simple(mut self, exporter: CaptureExporter) -> sdk::trace::Tracer {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut provider_builder = sdk::trace::TracerProvider::builder().with_simple_exporter(exporter);
        if let Some(config) = self.trace_config.take() {
            provider_builder = provider_builder.with_config(config);
        }
        let provider = provider_builder.build();
        let tracer = provider.tracer("opentelemetry");
        global::set_tracer_provider(provider);

        tracer
    }
}

/// A [`CaptureExporter`] that sends spans to stdout.
#[derive(Debug, Clone)]
pub struct CaptureExporter {
    logs: Arc<Mutex<HashMap<TraceId, Vec<SpanData>>>>,
}

impl CaptureExporter {
    pub fn new() -> Self {
        Self {
            logs: Default::default(),
        }
    }

    pub async fn capture(&self, trace_id: TraceId) {
        let mut logs = self.logs.lock().await;
        logs.insert(trace_id, Vec::new());
    }

    pub async fn get(&self, trace_id: TraceId) -> String {
        let mut logs = self.logs.lock().await;
        if let Some(spans) = logs.remove(&trace_id) {
            spans_to_json(&spans)
        } else {
            String::new()
        }
    }
}

impl Default for CaptureExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SpanExporter for CaptureExporter {
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let batch = batch.into_iter().filter(|span| span.name == "quaint:query");

        let mut logs = self.logs.lock().await;
        for span in batch {
            let trace_id = span.span_context.trace_id();

            if let Some(spans) = logs.get_mut(&trace_id) {
                spans.push(span)
            }
        }

        Ok(())
    }
}
