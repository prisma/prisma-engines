mod models;

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
use std::{collections::HashMap, sync::Arc};
use std::{fmt::Debug, time::Duration};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum Config {
    Enabled(ConfiguredCapturer),
    Disabled,
}

pub(crate) fn enabled(c: CaptureExporter, trace_id: TraceId) -> Config {
    Config::Enabled(ConfiguredCapturer { capturer: c, trace_id })
}

pub fn disabled() -> Config {
    Config::Disabled
}
/// A ConfiguredCapturer is ready to capture spans for a particular trace and is built from
#[derive(Debug, Clone)]
pub struct ConfiguredCapturer {
    capturer: CaptureExporter,
    trace_id: TraceId,
}

impl ConfiguredCapturer {
    pub async fn start_capturing(&self) {
        self.capturer
            .start_capturing(self.trace_id, CaptureTimeout::Default)
            .await
    }

    pub async fn fetch_captures(&self) -> Vec<models::ExportedSpan> {
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
    pub fn install(mut self, exporter: CaptureExporter) -> sdk::trace::Tracer {
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
pub struct CaptureExporter {
    pub(crate) traces: Arc<Mutex<HashMap<TraceId, Vec<models::ExportedSpan>>>>,
}

pub(crate) enum CaptureTimeout {
    #[allow(dead_code)]
    Duration(Duration),
    Default,
}

impl CaptureExporter {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1800);

    pub fn new() -> Self {
        Self {
            traces: Default::default(),
        }
    }

    pub(crate) async fn start_capturing(&self, trace_id: TraceId, timeout: CaptureTimeout) {
        let mut locked_traces = self.traces.lock().await;
        locked_traces.insert(trace_id, Vec::new());
        drop(locked_traces);

        let when = match timeout {
            CaptureTimeout::Duration(d) => d,
            CaptureTimeout::Default => Self::DEFAULT_TIMEOUT,
        };

        let traces = self.traces.clone();
        tokio::spawn(async move {
            tokio::time::sleep(when).await;
            let mut locked_traces = traces.lock().await;
            if locked_traces.remove(&trace_id).is_some() {
                warn!("Timeout waiting for spans to be captured. trace_id{}", trace_id)
            }
        });
    }

    pub(crate) async fn fetch_captures(&self, trace_id: TraceId) -> Vec<models::ExportedSpan> {
        let mut traces = self.traces.lock().await;

        if let Some(spans) = traces.remove(&trace_id) {
            spans
        } else {
            vec![]
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
        let mut traces = self.traces.lock().await;
        for span in batch {
            let trace_id = span.span_context.trace_id();

            if let Some(spans) = traces.get_mut(&trace_id) {
                spans.push(models::ExportedSpan::from(span))
            }
        }

        Ok(())
    }
}

// tests for capture exporter
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_garbage_collection() {
        let exporter = CaptureExporter::new();

        let trace_id = TraceId::from_hex("1").unwrap();
        let one_ms = Duration::from_millis(1);
        exporter
            .start_capturing(trace_id, CaptureTimeout::Duration(one_ms))
            .await;
        let traces = exporter.traces.lock().await;
        assert!(traces.get(&trace_id).is_some());
        drop(traces);

        tokio::time::sleep(10 * one_ms).await;

        let traces = exporter.traces.lock().await;
        assert!(traces.get(&trace_id).is_none());
    }
}
