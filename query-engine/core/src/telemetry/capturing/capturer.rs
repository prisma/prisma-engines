use super::{settings::Settings, storage::Storage};
use crate::models;
use opentelemetry::{
    sdk::{
        export::trace::SpanData,
        trace::{Span, SpanProcessor},
    },
    trace::{TraceId, TraceResult},
};
use std::{collections::HashMap, sync::Arc, sync::Mutex};

/// Capturer determines, based on a set of settings and a trace id, how capturing is going to be handled.
/// Generally, both the trace id and the settings will be derived from request headers. Thus, a new
/// value of this enum is created per request.
#[derive(Debug, Clone)]
pub enum Capturer {
    Enabled(Inner),
    Disabled,
}

impl Capturer {
    pub(super) fn new(processor: Processor, trace_id: TraceId, settings: Settings) -> Self {
        if settings.is_enabled() {
            return Self::Enabled(Inner {
                processor,
                trace_id,
                settings,
            });
        }

        Self::Disabled
    }
}

#[derive(Debug, Clone)]
pub struct Inner {
    pub(super) processor: Processor,
    pub(super) trace_id: TraceId,
    pub(super) settings: Settings,
}

impl Inner {
    pub async fn start_capturing(&self) {
        self.processor
            .start_capturing(self.trace_id, self.settings.clone())
            .await
    }

    pub async fn fetch_captures(&self) -> Option<Storage> {
        self.processor.fetch_captures(self.trace_id).await
    }
}

/// A [`SpanProcessor`] that captures and stores spans in memory in a synchronized dictionary for
/// later retrieval
#[derive(Debug, Clone)]
pub struct Processor {
    pub(crate) storage: Arc<Mutex<HashMap<TraceId, Storage>>>,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            storage: Default::default(),
        }
    }

    async fn start_capturing(&self, trace_id: TraceId, settings: Settings) {
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

    async fn fetch_captures(&self, trace_id: TraceId) -> Option<Storage> {
        let mut traces = self.storage.lock().unwrap();

        traces.remove(&trace_id)
    }
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

impl SpanProcessor for Processor {
    fn on_start(&self, _: &mut Span, _: &opentelemetry::Context) {
        // no-op
    }

    /// Exports a spancontaining zero or more events that might represent
    /// logs in the prisma client logging categories of logs (query, info, warn, error)
    ///
    /// There's an impedance between the client categories of logs and the server (standard)
    /// hierarchical levels of logs (trace, debug, info, warn, error).
    ///
    /// The most prominent difference is the "query" type of events. In the client these model
    /// database queries made by the engine through a connector. But ATM there's not a 1:1 mapping
    /// between the client "query" level and one of the server levels. And depending on the database
    /// mongo / relational, the information to build this kind of log event is logged diffeerently in
    /// the server.
    ///
    /// In the case of the of relational databaes --queried through sql_query_connector and eventually
    /// through quaint, a trace span describes the query-- `TraceSpan::represents_query_event`
    /// determines if a span represents a query event.
    ///
    /// In the case of mongo, an event represents the query, but it needs to be transformed before
    /// capturing it. `Event::query_event` does that.    
    fn on_end(&self, span_data: SpanData) {
        let trace_id = span_data.span_context.trace_id();

        let mut locked_storage = self.storage.lock().unwrap();
        if let Some(storage) = locked_storage.get_mut(&trace_id) {
            let settings = storage.settings.clone();

            let (events, span) = models::TraceSpan::from(span_data).split_events();

            if settings.traces_enabled() {
                storage.traces.push(span);
            }

            if storage.settings.logs_enabled() {
                events.into_iter().for_each(|log| {
                    let candidate = Candidate {
                        value: log,
                        settings: &settings,
                    };
                    if candidate.is_loggable_query_event() {
                        storage.logs.push(candidate.query_event())
                    } else if candidate.is_loggable_event() {
                        storage.logs.push(candidate.value)
                    }
                });
            }
        }
    }

    fn force_flush(&self) -> TraceResult<()> {
        // no-op
        Ok(())
    }

    fn shutdown(&mut self) -> TraceResult<()> {
        // no-op
        Ok(())
    }
}
const VALID_QUERY_ATTRS: [&str; 4] = ["query", "params", "target", "duration_ms"];
/// A Candidate represents either a span or an event that is being considered for capturing.
/// A Candidate can be converted into a [`Capture`].
#[derive(Debug, Clone)]
struct Candidate<'batch_iter> {
    value: models::LogEvent,
    settings: &'batch_iter Settings,
}

impl Candidate<'_> {
    #[inline(always)]
    fn is_loggable_query_event(&self) -> bool {
        if self.settings.included_log_levels.contains("query") {
            if let Some(target) = self.value.attributes.get("target") {
                if let Some(val) = target.as_str() {
                    return (val == "quaint::connector::metrics" && self.value.attributes.get("query").is_some())
                        || val == "mongodb_query_connector::query";
                }
            }
        }
        false
    }

    fn query_event(mut self) -> models::LogEvent {
        self.value
            .attributes
            .retain(|key, _| (&VALID_QUERY_ATTRS).contains(&key.as_str()));

        models::LogEvent {
            level: "query".to_string(),
            ..self.value
        }
    }

    #[inline(always)]
    fn is_loggable_event(&self) -> bool {
        self.settings.included_log_levels.contains(&self.value.level)
    }
}

/// tests for capture exporter
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_garbage_collection() {
        let exporter = Processor::new();

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
