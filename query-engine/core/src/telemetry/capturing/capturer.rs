use super::{settings::Settings, storage::Storage};
use crate::models;
use async_trait::async_trait;
use opentelemetry::{
    sdk::{
        export::trace::{ExportResult, SpanData, SpanExporter},
        trace::{BatchSpanProcessor, Span, SpanProcessor},
    },
    trace::{TraceId, TraceResult},
};
use std::{borrow::Cow, fmt, time::Duration};
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
        _ = super::processor().force_flush();
        let mut traces = self.storage.lock().unwrap();

        traces.remove(&trace_id)
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

/// A Candidate represents either a span or an event that is being considered for capturing.
/// A Candidate can be converted into a [`Capture`].
#[derive(Debug, Clone)]
struct Candidate<'batch_iter, T: Clone + fmt::Debug> {
    value: T,
    settings: &'batch_iter Settings,
    original_span_name: Option<Cow<'batch_iter, str>>,
}

impl Candidate<'_, models::TraceSpan> {
    #[inline(always)]
    fn is_loggable_quaint_query(&self) -> bool {
        self.settings.included_log_levels.contains("query")
            && self.original_span_name.is_some()
            && matches!(self.original_span_name, Some(Cow::Borrowed("quaint:query")))
    }

    fn query_event(&self) -> models::Event {
        let span = &self.value;

        let duration_ms = ((span.end_time[0] as f64 - span.start_time[0] as f64) * 1_000.0)
            + ((span.end_time[1] as f64 - span.start_time[1] as f64) / 1_000_000.0);

        let statement = if let Some(q) = span.attributes.get("db.statement") {
            match q {
                serde_json::Value::String(s) => s.to_string(),
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        };

        let attributes = vec![(
            "duration_ms".to_owned(),
            serde_json::Value::Number(serde_json::Number::from_f64(duration_ms).unwrap()),
        )]
        .into_iter()
        .collect();

        models::Event {
            span_id: Some(span.span_id.to_owned()),
            name: statement,
            level: "query".to_string(),
            timestamp: span.start_time,
            attributes,
        }
    }
}

impl Candidate<'_, models::LogEvent> {
    #[inline(always)]
    fn is_loggable_mongo_db_query(&self) -> bool {
        self.settings.included_log_levels.contains("query") && {
            if let Some(target) = self.value.attributes.get("target") {
                if let Some(val) = target.as_str() {
                    return val == "mongodb_query_connector::query";
                }
            }
            false
        }
    }

    #[inline(always)]
    fn is_loggable_event(&self) -> bool {
        self.settings.included_log_levels.contains(&self.value.level)
    }

    fn query_event(self) -> models::LogEvent {
        let mut attributes = self.value.attributes;
        let mut attrs = HashMap::new();
        if let Some(dur) = attributes.get("duration_ms") {
            attrs.insert("duration_ms".to_owned(), dur.clone());
        }

        let mut name = "uknown".to_owned();
        if let Some(query) = attributes.remove("query") {
            if let Some(str) = query.as_str() {
                name = str.to_owned();
            }
        }

        models::LogEvent {
            name,
            level: "query".to_string(),
            attributes: attrs,
            ..self.value
        }
    }
}

/// Capture provides mechanisms to transform a candidate into one of the enum variants.
/// This is necessary because a candidate span might also be transformed into a log event
/// (for quaint queries), or log events need to be transformed to a slightly different format
/// (for mongo queries). In addition some span and events are discarded.
enum Capture {
    Span(models::TraceSpan),
    LogEvent(models::LogEvent),
    Multiple(Vec<Capture>),
    Discarded,
}

impl Capture {
    /// Add the capture to the traces and logs vectors. We pass the vectors in to allow for
    /// a recursive implementation for the case of a candidate transforming into a Capture::Multiple
    fn add_to(self, traces: &mut Vec<models::TraceSpan>, logs: &mut Vec<models::LogEvent>) {
        match self {
            Capture::Span(span) => {
                traces.push(span);
            }
            Capture::LogEvent(log) => {
                logs.push(log);
            }
            Capture::Multiple(captures) => {
                for capture in captures {
                    capture.add_to(traces, logs);
                }
            }
            Capture::Discarded => {}
        }
    }
}

/// A Candidate Event can be transformed into either a slightly different LogEvent (for mongo queries)
/// be captrured as-is if its log level is among the levels to capture, or be discarded.
impl From<Candidate<'_, models::Event>> for Capture {
    fn from(candidate: Candidate<'_, models::Event>) -> Capture {
        if candidate.is_loggable_mongo_db_query() {
            // mongo events representing queries are transformed into a more meaningful log event
            Capture::LogEvent(candidate.query_event())
        } else if candidate.is_loggable_event() {
            Capture::LogEvent(candidate.value)
        } else {
            Capture::Discarded
        }
    }
}

/// A Candidate TraceSpan can be transformed into a LogEvent (for quaint queries) if query logging
/// is enabled; captured as-is, if tracing is enabled; or be discarded.
impl From<Candidate<'_, models::TraceSpan>> for Capture {
    fn from(candidate: Candidate<'_, models::TraceSpan>) -> Capture {
        let mut captures = vec![];

        if candidate.is_loggable_quaint_query() {
            captures.push(Capture::LogEvent(candidate.query_event()));
        }

        if candidate.settings.traces_enabled() {
            captures.push(Capture::Span(candidate.value));
        }

        match captures.len() {
            0 => Capture::Discarded,
            1 => captures.pop().unwrap(),
            _ => Capture::Multiple(captures),
        }
    }
}

#[async_trait]
impl SpanExporter for Exporter {
    /// Exports a batch of spans, each of them containing zero or more events that might represent
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
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        for span_data in batch {
            let trace_id = span_data.span_context.trace_id();

            let mut locked_storage = self.storage.lock().unwrap();
            if let Some(storage) = locked_storage.get_mut(&trace_id) {
                let settings = storage.settings.clone();
                let original_span_name = span_data.name.clone();

                let (events, span) = models::TraceSpan::from(span_data).split_events();

                let candidate_span = Candidate {
                    value: span,
                    settings: &settings,
                    original_span_name: Some(original_span_name),
                };

                let capture: Capture = candidate_span.into();
                capture.add_to(&mut storage.traces, &mut storage.logs);

                if storage.settings.logs_enabled() {
                    events.into_iter().for_each(|log| {
                        let candidate_event = Candidate {
                            value: log,
                            settings: &settings,
                            original_span_name: None,
                        };
                        let capture: Capture = candidate_event.into();
                        capture.add_to(&mut storage.traces, &mut storage.logs);
                    });
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
    fn on_start(&self, _: &mut Span, _: &opentelemetry::Context) {
        // no-op
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

    #[test]
    fn test_candidate_event_transformation() {
        let event = models::LogEvent {
            span_id: Some("00f067aa0ba902b7".to_owned()),
            name: "foo bar".to_owned(),
            level: "debug".to_owned(),
            timestamp: [101, 0],
            attributes: vec![
                (
                    "target".to_owned(),
                    serde_json::Value::String("mongodb_query_connector::query".to_owned()),
                ),
                (
                    "query".to_owned(),
                    serde_json::Value::String("db.Users.find()".to_owned()),
                ),
                ("duration_ms".to_owned(), serde_json::json!(100.0)),
            ]
            .into_iter()
            .collect(),
        };

        let only_query_log_events: Settings = "query".into();

        let candidate = Candidate {
            value: event.clone(),
            settings: &only_query_log_events,
            original_span_name: None,
        };

        let capture: Capture = candidate.into();
        match capture {
            Capture::LogEvent(event) => {
                assert_eq!(event.level, "query");
                assert_eq!(event.name.to_string().as_str(), "db.Users.find()");
                assert_eq!(event.attributes.get("duration_ms").unwrap().to_string(), "100.0");
            }
            _ => unreachable!(),
        };

        let event = models::LogEvent {
            attributes: vec![(
                "target".to_owned(),
                serde_json::Value::String("a different one".to_owned()),
            )]
            .into_iter()
            .collect(),
            ..event
        };
        let candidate = Candidate {
            value: event.clone(),
            settings: &only_query_log_events,
            original_span_name: None,
        };

        let capture: Capture = candidate.into();
        match capture {
            Capture::Discarded => {}
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_candidate_span_transformation() {
        let trace_span = models::TraceSpan {
            trace_id: "4bf92f3577b34da6a3ce929d0e0e4736".to_owned(),
            span_id: "00f067aa0ba902b7".to_owned(),
            parent_span_id: "00f067aa0ba902b5".to_owned(),
            name: "prisma:engine:db_query".to_ascii_lowercase(),
            start_time: [101, 0],
            end_time: [101, 10000000],
            attributes: vec![(
                "db.statement".to_owned(),
                serde_json::Value::String("SELECT 1".to_owned()),
            )]
            .into_iter()
            .collect(),
            events: Default::default(),
            links: Default::default(),
        };

        // capturing query events
        let only_query_log_events: Settings = "query".into();
        let original_span_name = Some(Cow::Borrowed("quaint:query"));

        let candidate = Candidate {
            value: trace_span.clone(),
            settings: &only_query_log_events,
            original_span_name: original_span_name.clone(),
        };

        let capture: Capture = candidate.into();
        match capture {
            Capture::LogEvent(event) => {
                assert_eq!(event.level, "query");
                assert_eq!(event.name.to_string().as_str(), "SELECT 1");
                assert_eq!(event.attributes.get("duration_ms").unwrap().to_string(), "10.0");
            }
            _ => unreachable!(),
        };

        // capturing query and tracing events
        let query_logs_and_traces: Settings = "query, tracing".into();
        let candidate = Candidate {
            value: trace_span.clone(),
            settings: &query_logs_and_traces,
            original_span_name: original_span_name.clone(),
        };

        let capture: Capture = candidate.into();
        match capture {
            Capture::Multiple(captures) => {
                match captures[0] {
                    Capture::LogEvent(_) => {}
                    _ => unreachable!(),
                };

                match captures[1] {
                    Capture::Span(_) => {}
                    _ => unreachable!(),
                };
            }
            _ => unreachable!(),
        };

        // capturing nothing
        let reject_all: Settings = "".into();
        let candidate = Candidate {
            value: trace_span.clone(),
            settings: &reject_all,
            original_span_name: original_span_name.clone(),
        };

        let capture: Capture = candidate.into();
        match capture {
            Capture::Discarded => {}
            _ => unreachable!(),
        };
    }

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
