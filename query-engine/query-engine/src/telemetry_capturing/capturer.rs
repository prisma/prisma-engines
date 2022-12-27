use async_trait::async_trait;
use opentelemetry::{
    sdk::{
        export::trace::{ExportResult, SpanData, SpanExporter},
        trace::{BatchSpanProcessor, Span, SpanProcessor},
    },
    trace::{TraceId, TraceResult},
};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc, sync::Mutex};

use super::{models, settings::Settings, storage::Storage};

/// Capturer determines, based on a set of settings and a trace id, how capturing is going to be handled.
/// Generally, both the trace id and the settings will be derived from request headers. Thus, a new
/// value of this enum is created per request.
#[derive(Debug, Clone)]
pub enum Capturer {
    Enabled(Inner),
    Disabled,
}

impl Capturer {
    pub(super) fn new(exporter: Exporter, trace_id: TraceId, settings: Settings) -> Self {
        if settings.is_enabled() {
            return Self::Enabled(Inner {
                exporter,
                trace_id,
                settings,
            });
        }

        Self::Disabled
    }
}

#[derive(Debug, Clone)]
pub struct Inner {
    pub(super) exporter: Exporter,
    pub(super) trace_id: TraceId,
    pub(super) settings: Settings,
}

impl Inner {
    pub async fn start_capturing(&self) {
        self.exporter
            .start_capturing(self.trace_id, self.settings.clone())
            .await
    }

    pub async fn fetch_captures(&self) -> Option<Storage> {
        self.exporter.fetch_captures(self.trace_id).await
    }
}

/// A [`SpanExporter`] that captures and stores spans in memory in a synchronized dictionary for
/// later retrieval
#[derive(Debug, Clone)]
pub struct Exporter {
    pub(crate) storage: Arc<Mutex<HashMap<TraceId, Storage>>>,
}

impl Exporter {
    pub fn new() -> Self {
        Self {
            storage: Default::default(),
        }
    }

    pub(self) async fn start_capturing(&self, trace_id: TraceId, settings: Settings) {
        let mut locked_storage = self.storage.lock().unwrap();
        locked_storage.insert(trace_id, settings.clone().into());
        drop(locked_storage);

        let ttl = settings.ttl;
        let storage = self.storage.clone();
        tokio::spawn(async move {
            tokio::time::sleep(ttl).await;
            let mut locked_traces = storage.lock().unwrap();
            if locked_traces.remove(&trace_id).is_some() {
                warn!("Timeout waiting for telemetry to be captured. trace_id={}", trace_id)
            }
        });
    }

    pub(self) async fn fetch_captures(&self, trace_id: TraceId) -> Option<Storage> {
        _ = super::global_processor().force_flush();
        let mut traces = self.storage.lock().unwrap();

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
        for span in batch {
            let trace_id = span.span_context.trace_id();

            let mut locked_storage = self.storage.lock().unwrap();
            if let Some(storage) = locked_storage.get_mut(&trace_id) {
                let trace = models::ExportedSpan::from(span);

                if storage.settings.included_log_levels.contains("query") && trace.is_query() {
                    storage.logs.push(trace.query_event())
                }

                let (logs, trace) = trace.split_logs();

                if storage.settings.logs_enabled() {
                    logs.into_iter()
                        .filter(|l| storage.settings.included_log_levels.contains(&l.level))
                        .for_each(|l| storage.logs.push(l));
                }

                if storage.settings.traces_enabled() {
                    storage.traces.push(trace);
                }
            }
        }

        Ok(())
    }
}

/// An adapter of a SpanProcessor that is shareable accross thread boundaries, so we can
/// flush the processor before each request finishes.
#[derive(Debug, Clone)]
pub(super) struct SyncedSpanProcessor(Arc<Mutex<dyn SpanProcessor>>);

impl SyncedSpanProcessor {
    pub(super) fn new(exporter: Exporter) -> Self {
        let adaptee = BatchSpanProcessor::builder(exporter, opentelemetry::runtime::Tokio)
            .with_scheduled_delay(Duration::new(0, 1))
            .build();
        Self(Arc::new(Mutex::new(adaptee)))
    }
}

impl SpanProcessor for SyncedSpanProcessor {
    fn on_start(&self, span: &mut Span, cx: &opentelemetry::Context) {
        self.0.lock().unwrap().on_start(span, cx)
    }

    fn on_end(&self, span: SpanData) {
        self.0.lock().unwrap().on_end(span)
    }

    fn force_flush(&self) -> TraceResult<()> {
        self.0.lock().unwrap().force_flush()
    }

    fn shutdown(&mut self) -> TraceResult<()> {
        self.0.lock().unwrap().shutdown()
    }
}

/// tests for capture exporter
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_garbage_collection() {
        let exporter = Exporter::new();

        let trace_id = TraceId::from_hex("1").unwrap();
        let one_ms = Duration::from_millis(1);

        let mut settings = Settings::default();
        settings.ttl = one_ms;

        exporter.start_capturing(trace_id, settings).await;
        let storage = exporter.storage.lock().unwrap();
        assert!(storage.get(&trace_id).is_some());
        drop(storage);

        tokio::time::sleep(10 * one_ms).await;

        let storage = exporter.storage.lock().unwrap();
        assert!(storage.get(&trace_id).is_none());
    }
}
