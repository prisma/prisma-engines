use std::{borrow::Cow, fmt::Debug, str::FromStr};

use ahash::{HashMap, HashMapExt};
use enumflags2::{BitFlags, bitflags};
use serde::Serialize;
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot,
};

use crate::collector::{AllowAttribute, CollectedEvent, CollectedSpan, Collector};
use crate::id::{RequestId, SpanId};
use crate::models::{LogLevel, SpanKind};
use crate::time::HrTime;

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
    target: &'static str,
    level: LogLevel,
    timestamp: HrTime,
    attributes: HashMap<&'static str, serde_json::Value>,
}

impl From<CollectedEvent> for ExportedEvent {
    fn from(event: CollectedEvent) -> Self {
        Self {
            span_id: event.span_id,
            target: event.target,
            level: event.level,
            timestamp: event.timestamp.into(),
            attributes: event.attributes,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TraceData {
    pub spans: Vec<ExportedSpan>,
    pub events: Vec<ExportedEvent>,
}

struct Trace {
    data: TraceData,
    settings: CaptureSettings,
}

impl Trace {
    fn new(settings: CaptureSettings) -> Self {
        Self {
            data: TraceData::default(),
            settings,
        }
    }
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

#[derive(Debug, Clone, Default)]
pub struct CaptureSettings {
    targets: BitFlags<CaptureTarget>,
}

impl CaptureSettings {
    pub fn new(targets: impl Into<BitFlags<CaptureTarget>>) -> Self {
        Self {
            targets: targets.into(),
        }
    }

    pub fn none() -> Self {
        Self::new(BitFlags::empty())
    }

    pub fn filter(&self, target: CaptureTarget) -> bool {
        self.targets.contains(target)
    }
}

impl FromStr for CaptureSettings {
    type Err = ();

    fn from_str(targets: &str) -> Result<Self, Self::Err> {
        let mut flags = BitFlags::empty();

        for target in targets.split(',') {
            let target = target.trim();
            flags |= target.parse::<CaptureTarget>()?;
        }

        Ok(CaptureSettings::new(flags))
    }
}

enum Message {
    StartCapturing(RequestId, CaptureSettings),
    StopCapturing(RequestId, oneshot::Sender<Option<TraceData>>),
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
                    Message::StartCapturing(request_id, settings) => {
                        traces.insert(request_id, Trace::new(settings));
                    }

                    Message::StopCapturing(request_id, tx) => {
                        _ = tx.send(traces.remove(&request_id).map(|trace| trace.data));
                    }

                    Message::AddSpan(request_id, span) => {
                        if let Some(trace) = traces.get_mut(&request_id)
                            && trace.settings.filter(CaptureTarget::Spans)
                        {
                            trace.data.spans.push(span.into());
                        }
                    }

                    Message::AddEvent(request_id, event) => {
                        if let Some(trace) = traces.get_mut(&request_id)
                            && trace.settings.filter(event.level.into())
                        {
                            trace.data.events.push(event.into());
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn start_capturing(&self, request_id: RequestId, settings: CaptureSettings) -> RequestId {
        self.tx
            .send(Message::StartCapturing(request_id, settings))
            .expect("capturer task panicked");

        request_id
    }

    pub async fn stop_capturing(&self, request_id: RequestId) -> Option<TraceData> {
        let (tx, rx) = oneshot::channel();

        self.tx
            .send(Message::StopCapturing(request_id, tx))
            .expect("capturer task panicked");

        rx.await.expect("capturer task dropped the sender")
    }
}

impl Collector for Exporter {
    type AttributeFilter = InternalAttributesFilter;

    fn add_span(&self, trace: RequestId, span: CollectedSpan) {
        self.tx
            .send(Message::AddSpan(trace, span))
            .expect("capturer task panicked");
    }

    fn add_event(&self, trace: RequestId, event: CollectedEvent) {
        self.tx
            .send(Message::AddEvent(trace, event))
            .expect("capturer task panicked");
    }
}

pub struct InternalAttributesFilter;

impl AllowAttribute for InternalAttributesFilter {
    fn allow_on_span(name: &'static str) -> bool {
        name != "user_facing"
    }

    fn allow_on_event(_name: &'static str) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use crate::NextId;

    use super::*;

    use CaptureTarget::*;

    fn capture_all() -> CaptureSettings {
        CaptureSettings::new(Spans | TraceEvents | DebugEvents | InfoEvents | WarnEvents | ErrorEvents | QueryEvents)
    }

    fn capture_spans() -> CaptureSettings {
        CaptureSettings::new(Spans)
    }

    #[test]
    fn test_capture_settings_from_str() {
        assert_eq!(
            "tracing".parse::<CaptureSettings>().unwrap().targets,
            BitFlags::from_flag(Spans)
        );
        assert_eq!(
            "trace".parse::<CaptureSettings>().unwrap().targets,
            BitFlags::from_flag(TraceEvents)
        );
        assert_eq!(
            "debug".parse::<CaptureSettings>().unwrap().targets,
            BitFlags::from_flag(DebugEvents)
        );
        assert_eq!(
            "info".parse::<CaptureSettings>().unwrap().targets,
            BitFlags::from_flag(InfoEvents)
        );
        assert_eq!(
            "warn".parse::<CaptureSettings>().unwrap().targets,
            BitFlags::from_flag(WarnEvents)
        );
        assert_eq!(
            "error".parse::<CaptureSettings>().unwrap().targets,
            BitFlags::from_flag(ErrorEvents)
        );
        assert_eq!(
            "query".parse::<CaptureSettings>().unwrap().targets,
            BitFlags::from_flag(QueryEvents)
        );

        let all = "tracing,trace,debug,info,warn,error,query"
            .parse::<CaptureSettings>()
            .unwrap();
        assert_eq!(all.targets, capture_all().targets);
    }

    #[tokio::test]
    async fn test_export_capture_cycle() {
        let exporter = Exporter::new();
        let request_id = exporter.start_capturing(RequestId::next(), capture_all()).await;

        let span = CollectedSpan {
            id: SpanId::try_from(1).unwrap(),
            parent_id: None,
            name: "test_span".into(),
            start_time: SystemTime::UNIX_EPOCH,
            duration: Duration::from_secs(1),
            kind: SpanKind::Internal,
            attributes: HashMap::new(),
            links: Vec::new(),
        };

        let event = CollectedEvent {
            span_id: span.id,
            target: "test_event",
            level: LogLevel::Info,
            timestamp: SystemTime::UNIX_EPOCH,
            attributes: HashMap::new(),
        };

        exporter.add_span(request_id, span.clone());
        exporter.add_event(request_id, event.clone());

        let trace = exporter.stop_capturing(request_id).await.unwrap();

        insta::assert_ron_snapshot!(trace, @r#"
        TraceData(
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
              target: "test_event",
              level: info,
              timestamp: HrTime(0, 0),
              attributes: {},
            ),
          ],
        )
        "#);
    }

    #[tokio::test]
    async fn test_export_capture_cycle_with_ignored() {
        let exporter = Exporter::new();
        let request_id = exporter.start_capturing(RequestId::next(), capture_spans()).await;

        let span = CollectedSpan {
            id: SpanId::try_from(1).unwrap(),
            parent_id: None,
            name: "test_span".into(),
            start_time: SystemTime::UNIX_EPOCH,
            duration: Duration::from_secs(1),
            kind: SpanKind::Internal,
            attributes: HashMap::new(),
            links: Vec::new(),
        };

        let event = CollectedEvent {
            span_id: span.id,
            target: "test_event",
            level: LogLevel::Info,
            timestamp: SystemTime::UNIX_EPOCH,
            attributes: HashMap::new(),
        };

        exporter.add_span(request_id, span.clone());
        exporter.add_event(request_id, event.clone());

        let trace = exporter.stop_capturing(request_id).await.unwrap();

        insta::assert_ron_snapshot!(trace, @r#"
        TraceData(
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
          events: [],
        )
        "#);
    }
}
