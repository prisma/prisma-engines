use std::{borrow::Cow, collections::HashMap, fmt::Debug, str::FromStr, sync::Arc};

use enumflags2::{bitflags, BitFlags};
use serde::Serialize;
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot,
};

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

#[derive(Debug, Clone, Default, Serialize)]
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

enum Message {
    StartCapturing(RequestId),
    StopCapturing(RequestId, oneshot::Sender<Option<Trace>>),
    AddSpan(RequestId, CollectedSpan),
    AddEvent(RequestId, CollectedEvent),
}

#[derive(Clone)]
pub struct Exporter {
    tx: UnboundedSender<Message>,
}

impl Debug for Exporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Exporter").finish()
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel();

        crosstarget_utils::task::spawn(async move {
            let mut traces = HashMap::new();

            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::StartCapturing(request_id) => {
                        traces.insert(request_id, Trace::default());
                    }

                    Message::StopCapturing(request_id, tx) => {
                        _ = tx.send(traces.remove(&request_id));
                    }

                    Message::AddSpan(request_id, span) => {
                        if let Some(trace) = traces.get_mut(&request_id) {
                            trace.spans.push(span.into());
                        }
                    }

                    Message::AddEvent(request_id, event) => {
                        if let Some(trace) = traces.get_mut(&request_id) {
                            trace.events.push(event.into());
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn start_capturing(&self) -> RequestId {
        let request_id = RequestId::next();
        _ = self.tx.send(Message::StartCapturing(request_id));
        request_id
    }

    pub async fn stop_capturing(&self, request_id: RequestId) -> Option<Trace> {
        let (tx, rx) = oneshot::channel();
        _ = self.tx.send(Message::StopCapturing(request_id, tx));
        rx.await.expect("capturer task dropped the sender")
    }
}

impl Collector for Exporter {
    fn add_span(&self, trace: RequestId, span: CollectedSpan) {
        _ = self.tx.send(Message::AddSpan(trace, span));
    }

    fn add_event(&self, trace: RequestId, event: CollectedEvent) {
        _ = self.tx.send(Message::AddEvent(trace, event));
    }
}
#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::*;

    #[tokio::test]
    async fn test_export_capture_cycle() {
        let exporter = Exporter::new();
        let request_id = exporter.start_capturing().await;

        let span = CollectedSpan {
            id: tracing::span::Id::from_u64(1).into(),
            parent_id: None,
            name: "test_span".into(),
            start_time: SystemTime::UNIX_EPOCH.into(),
            duration: Duration::from_secs(1).into(),
            kind: SpanKind::Internal,
            attributes: HashMap::new(),
            links: Vec::new(),
        };

        let event = CollectedEvent {
            span_id: span.id,
            name: "test_event",
            level: LogLevel::Info,
            timestamp: SystemTime::UNIX_EPOCH.into(),
            attributes: HashMap::new(),
        };

        exporter.add_span(request_id, span.clone());
        exporter.add_event(request_id, event.clone());

        let trace = exporter.stop_capturing(request_id).await.unwrap();

        insta::assert_ron_snapshot!(trace, @r#"
        Trace(
          spans: [
            ExportedSpan(
              id: SpanId("1"),
              parentId: None,
              name: "test_span",
              startTime: HrTime(0, 0),
              endTime: HrTime(1, 0),
              kind: internal,
            ),
          ],
          events: [
            ExportedEvent(
              spanId: SpanId("1"),
              name: "test_event",
              level: Info,
              timestamp: HrTime(0, 0),
              attributes: {},
            ),
          ],
        )
        "#);
    }

    #[test]
    fn test_capture_target_from_log_level() {
        assert_eq!(CaptureTarget::from(LogLevel::Trace), CaptureTarget::TraceEvents);
        assert_eq!(CaptureTarget::from(LogLevel::Debug), CaptureTarget::DebugEvents);
        assert_eq!(CaptureTarget::from(LogLevel::Info), CaptureTarget::InfoEvents);
        assert_eq!(CaptureTarget::from(LogLevel::Warn), CaptureTarget::WarnEvents);
        assert_eq!(CaptureTarget::from(LogLevel::Error), CaptureTarget::ErrorEvents);
        assert_eq!(CaptureTarget::from(LogLevel::Query), CaptureTarget::QueryEvents);
    }

    #[test]
    fn test_capture_target_from_str() {
        assert_eq!("tracing".parse::<CaptureTarget>().unwrap(), CaptureTarget::Spans);
        assert_eq!("trace".parse::<CaptureTarget>().unwrap(), CaptureTarget::TraceEvents);
        assert_eq!("debug".parse::<CaptureTarget>().unwrap(), CaptureTarget::DebugEvents);
        assert_eq!("info".parse::<CaptureTarget>().unwrap(), CaptureTarget::InfoEvents);
        assert_eq!("warn".parse::<CaptureTarget>().unwrap(), CaptureTarget::WarnEvents);
        assert_eq!("error".parse::<CaptureTarget>().unwrap(), CaptureTarget::ErrorEvents);
        assert_eq!("query".parse::<CaptureTarget>().unwrap(), CaptureTarget::QueryEvents);
    }
}
