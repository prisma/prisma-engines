use std::{borrow::Cow, collections::HashMap, str::FromStr, sync::Arc};

use enumflags2::{bitflags, BitFlags};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};

use crate::models::{HrTime, LogLevel, SpanKind};

use super::collector::{CollectedEvent, CollectedSpan, Collector, RequestId, SpanId};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedSpan {
    id: SpanId,
    parent_id: Option<SpanId>,
    name: Cow<'static, str>,
    start_time: HrTime,
    end_time: HrTime,
    kind: SpanKind,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    attributes: HashMap<&'static str, serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    links: Vec<SpanId>,
}

impl From<CollectedSpan> for ExportedSpan {
    fn from(span: CollectedSpan) -> Self {
        Self {
            id: span.id,
            parent_id: span.parent_id,
            name: span.name,
            start_time: span.start_time.into(),
            end_time: (span.start_time + span.duration).into(),
            kind: span.kind,
            attributes: span.attributes,
            links: span.links,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedEvent {
    span_id: SpanId,
    name: &'static str,
    level: LogLevel,
    timestamp: HrTime,
    attributes: HashMap<&'static str, serde_json::Value>,
}

impl From<CollectedEvent> for ExportedEvent {
    fn from(event: CollectedEvent) -> Self {
        Self {
            span_id: event.span_id,
            name: event.name,
            level: event.level,
            timestamp: event.timestamp.into(),
            attributes: event.attributes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Trace {
    pub spans: Vec<ExportedSpan>,
    pub events: Vec<ExportedEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[bitflags]
#[repr(u8)]
pub enum CaptureTarget {
    TraceEvents,
    DebugEvents,
    InfoEvents,
    WarnEvents,
    ErrorEvents,
    QueryEvents,
    Spans,
}

impl From<LogLevel> for CaptureTarget {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => Self::TraceEvents,
            LogLevel::Debug => Self::DebugEvents,
            LogLevel::Info => Self::InfoEvents,
            LogLevel::Warn => Self::WarnEvents,
            LogLevel::Error => Self::ErrorEvents,
            LogLevel::Query => Self::QueryEvents,
        }
    }
}

impl FromStr for CaptureTarget {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tracing" => Ok(Self::Spans),
            _ => Ok(s.parse::<LogLevel>()?.into()),
        }
    }
}

pub struct CaptureSettings {
    targets: BitFlags<CaptureTarget>,
}

#[derive(Clone)]
pub struct Exporter(Arc<Inner>);

struct Inner {
    // We use fine-grained locking here to avoid contention. On any operations with the existing
    // traces, the outer lock should only be held for a tiny amount of time to clone the inner Arc.
    traces: RwLock<HashMap<RequestId, Arc<Mutex<Trace>>>>,
}

impl Exporter {
    pub fn new() -> Self {
        Self(Arc::new(Inner {
            traces: RwLock::new(HashMap::new()),
        }))
    }

    pub async fn start_capturing(&self) -> RequestId {
        let request_id = RequestId::next();

        self.0.traces.write().await.insert(
            request_id,
            Arc::new(Mutex::new(Trace {
                spans: Vec::new(),
                events: Vec::new(),
            })),
        );

        request_id
    }

    pub async fn stop_capturing(&self, request_id: RequestId) -> Option<Trace> {
        let trace = self.0.traces.write().await.remove(&request_id)?;

        Some(match Arc::try_unwrap(trace) {
            Ok(trace) => trace.into_inner(),
            Err(trace) => trace.lock().await.clone(),
        })
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for Exporter {
    fn add_span(&self, trace: RequestId, span: CollectedSpan) {
        let inner = Arc::clone(&self.0);

        tokio::spawn(async move {
            let trace = inner.traces.read().await.get(&trace).cloned();

            if let Some(trace) = trace {
                let span = span.into();
                trace.lock().await.spans.push(span);
            }
        });
    }

    fn add_event(&self, trace: RequestId, event: CollectedEvent) {
        let inner = Arc::clone(&self.0);

        tokio::spawn(async move {
            let trace = inner.traces.read().await.get(&trace).cloned();

            if let Some(trace) = trace {
                let event = event.into();
                trace.lock().await.events.push(event);
            }
        });
    }
}
