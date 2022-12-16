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
use query_core::UserFacingSpan;
use std::{collections::HashMap, sync::Arc};
use std::{fmt::Debug, time::Duration};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum Config {
    Enabled(ConfiguredCapturer),
    Disabled,
}

pub(crate) fn enabled(c: TraceCapturer, trace_id: TraceId) -> Config {
    Config::Enabled(ConfiguredCapturer { capturer: c, trace_id })
}

pub fn disabled() -> Config {
    Config::Disabled
}
/// A ConfiguredCapturer is ready to capture spans for a particular trace and is built from
#[derive(Debug, Clone)]
pub struct ConfiguredCapturer {
    capturer: TraceCapturer,
    trace_id: TraceId,
}

impl ConfiguredCapturer {
    pub async fn start_capturing(&self) {
        self.capturer.start_capturing(self.trace_id).await
    }

    pub async fn fetch_captures(&self) -> Vec<UserFacingSpan> {
        self.capturer.fetch_captures(self.trace_id).await
    }
}

/// Pipeline builder
#[derive(Debug)]
pub struct PipelineBuilder {
    trace_config: Option<sdk::trace::Config>,
}

/// Create a new in memory expoter
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
    pub fn install(mut self, exporter: TraceCapturer) -> sdk::trace::Tracer {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let processor = sdk::trace::BatchSpanProcessor::builder(exporter, opentelemetry::runtime::Tokio)
            .with_scheduled_delay(Duration::new(0, 1))
            .build();
        let mut provider_builder = sdk::trace::TracerProvider::builder().with_span_processor(processor);

        if let Some(config) = self.trace_config.take() {
            provider_builder = provider_builder.with_config(config);
        }
        let provider = provider_builder.build();
        let tracer = provider.tracer("opentelemetry");
        global::set_tracer_provider(provider);

        tracer
    }
}

/// A [`SpanExporter`] that captures and stores spans in memory in a synchronized dictionary for
/// later retrieval
#[derive(Debug, Clone)]
pub struct TraceCapturer {
    traces: Arc<Mutex<HashMap<TraceId, Vec<SpanData>>>>,
}

impl TraceCapturer {
    pub fn new(enable: bool) -> Option<Self> {
        if !enable {
            return None;
        }

        Some(Self {
            traces: Default::default(),
        })
    }

    pub async fn start_capturing(&self, trace_id: TraceId) {
        let mut traces = self.traces.lock().await;
        traces.insert(trace_id, Vec::new());
    }

    pub async fn fetch_captures(&self, trace_id: TraceId) -> Vec<UserFacingSpan> {
        let mut traces = self.traces.lock().await;

        match traces.remove(&trace_id) {
            Some(spans) => spans.iter().map(UserFacingSpan::from).collect(),
            None => vec![],
        }
    }
}

#[async_trait]
impl SpanExporter for TraceCapturer {
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let mut traces = self.traces.lock().await;
        for span in batch {
            let trace_id = span.span_context.trace_id();

            if let Some(spans) = traces.get_mut(&trace_id) {
                spans.push(span)
            }
        }

        Ok(())
    }
}
