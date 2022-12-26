use async_trait::async_trait;
use opentelemetry::{
    global,
    sdk::{
        self,
        export::trace::{ExportResult, SpanData, SpanExporter},
        propagation::TraceContextPropagator,
        trace::Tracer,
    },
    trace::{TraceId, TracerProvider},
};
use std::{collections::HashMap, sync::Arc};
use std::{fmt::Debug, time::Duration};
use tokio::sync::Mutex;

use super::models;

// Installs an opentelemetry tracer globally, which is configured to proecss
// spans and export them to the provided exporter.
pub fn setup_and_install_tracer_globally(exporter: Exporter) -> Tracer {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let processor = sdk::trace::BatchSpanProcessor::builder(exporter, opentelemetry::runtime::Tokio)
        .with_scheduled_delay(Duration::new(0, 1))
        .build();
    let provider_builder = sdk::trace::TracerProvider::builder().with_span_processor(processor);

    let provider = provider_builder.build();
    let tracer = provider.tracer("opentelemetry");
    global::set_tracer_provider(provider);

    tracer
}

#[derive(Debug, Clone)]
pub enum Config {
    Enabled(ConfiguredCapturer),
    Disabled,
}

pub fn enabled(c: Exporter, trace_id: TraceId) -> Config {
    Config::Enabled(ConfiguredCapturer { capturer: c, trace_id })
}

pub fn disabled() -> Config {
    Config::Disabled
}
/// A ConfiguredCapturer is ready to capture spans for a particular trace and is built from
#[derive(Debug, Clone)]
pub struct ConfiguredCapturer {
    capturer: Exporter,
    trace_id: TraceId,
}

impl ConfiguredCapturer {
    pub async fn start_capturing(&self) {
        self.capturer
            .start_capturing(self.trace_id, CaptureTimeout::Default)
            .await
    }

    pub async fn fetch_captures(&self) -> Option<TelemetryStorage> {
        self.capturer.fetch_captures(self.trace_id).await
    }
}

/// A [`SpanExporter`] that captures and stores spans in memory in a synchronized dictionary for
/// later retrieval
#[derive(Debug, Clone)]
pub struct Exporter {
    pub(crate) storage: Arc<Mutex<HashMap<TraceId, TelemetryStorage>>>,
}

#[derive(Debug, Default)]
pub struct TelemetryStorage {
    pub traces: Vec<models::ExportedSpan>,
    pub logs: Vec<models::ExportedSpan>,
}

pub(crate) enum CaptureTimeout {
    #[allow(dead_code)]
    Duration(Duration),
    Default,
}

impl Exporter {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1800);

    pub fn new() -> Self {
        Self {
            storage: Default::default(),
        }
    }

    pub(crate) async fn start_capturing(&self, trace_id: TraceId, timeout: CaptureTimeout) {
        let mut locked_storage = self.storage.lock().await;
        locked_storage.insert(trace_id, Default::default());
        drop(locked_storage);

        let when = match timeout {
            CaptureTimeout::Duration(d) => d,
            CaptureTimeout::Default => Self::DEFAULT_TIMEOUT,
        };

        let storage = self.storage.clone();
        tokio::spawn(async move {
            tokio::time::sleep(when).await;
            let mut locked_traces = storage.lock().await;
            if locked_traces.remove(&trace_id).is_some() {
                warn!("Timeout waiting for telemetry to be captured. trace_id={}", trace_id)
            }
        });
    }

    pub(crate) async fn fetch_captures(&self, trace_id: TraceId) -> Option<TelemetryStorage> {
        let mut traces = self.storage.lock().await;

        traces.remove(&trace_id)
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SpanExporter for Exporter {
    // todo: lock less
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let mut locked_storage = self.storage.lock().await;
        for span in batch {
            let trace_id = span.span_context.trace_id();

            if let Some(trace_storage) = locked_storage.get_mut(&trace_id) {
                trace_storage.traces.push(models::ExportedSpan::from(span))
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
        let exporter = Exporter::new();

        let trace_id = TraceId::from_hex("1").unwrap();
        let one_ms = Duration::from_millis(1);
        exporter
            .start_capturing(trace_id, CaptureTimeout::Duration(one_ms))
            .await;
        let traces = exporter.storage.lock().await;
        assert!(traces.get(&trace_id).is_some());
        drop(traces);

        tokio::time::sleep(10 * one_ms).await;

        let traces = exporter.storage.lock().await;
        assert!(traces.get(&trace_id).is_none());
    }
}
