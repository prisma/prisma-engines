use std::{
    borrow::Cow,
    collections::HashMap,
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};

use derive_more::Display;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use tracing::Level;

use crate::models::{LogLevel, SpanKind, TraceSpan};

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(fmt = "{}", _0)]
#[repr(transparent)]
struct SerializableNonZeroU64(NonZeroU64);

impl SerializableNonZeroU64 {
    pub fn into_u64(self) -> u64 {
        self.0.get()
    }

    pub fn from_u64(value: u64) -> Option<Self> {
        NonZeroU64::new(value).map(Self)
    }
}

impl Serialize for SerializableNonZeroU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as string to preserve full u64 precision in JavaScript. Otherwise values
        // larger than 2^53 - 1 will be parsed as floats on the client side, making it possible for
        // IDs to collide.
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SerializableNonZeroU64 {
    fn deserialize<D>(deserializer: D) -> Result<SerializableNonZeroU64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let value = value.parse().map_err(serde::de::Error::custom)?;
        Ok(SerializableNonZeroU64(
            NonZeroU64::new(value).ok_or_else(|| serde::de::Error::custom("value must be non-zero"))?,
        ))
    }
}

/// A unique identifier for a span. It maps directly to [`tracing::span::Id`] assigned by
/// [`tracing_subscriber::registry::Registry`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[repr(transparent)]
pub struct SpanId(SerializableNonZeroU64);

impl From<&tracing::span::Id> for SpanId {
    fn from(id: &tracing::span::Id) -> Self {
        Self(SerializableNonZeroU64(id.into_non_zero_u64()))
    }
}

impl From<tracing::span::Id> for SpanId {
    fn from(id: tracing::span::Id) -> Self {
        Self::from(&id)
    }
}

/// A unique identifier for an engine trace, representing a tree of spans. These internal traces *do
/// not* correspond to OpenTelemetry traces defined by [`crate::capturing::ng::traceparent::TraceParent`].
/// One OpenTelemetry trace may contain multiple Prisma Client operations, each of them leading to
/// one or more engine requests. Since engine traces map 1:1 to requests to the engine, we call
/// these trace IDs "request IDs" to disambiguate and avoid confusion.
///
/// We don't use IDs of the root spans themselves for this purpose because span IDs are only
/// guaranteed to be unique among the spans active at the same time. They may be reused after a
/// span is closed, so they are not historically unique. We store the collected spans and events
/// for some short time after the spans are closed until the client requests them, so we need
/// request IDs that are guaranteed to be unique for a very long period of time (although they
/// still don't necessarily have to be unique for the whole lifetime of the process).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[repr(transparent)]
pub struct RequestId(SerializableNonZeroU64);

impl RequestId {
    pub fn next() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let mut id = 0;
        while id == 0 {
            id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        }

        Self(SerializableNonZeroU64(NonZeroU64::new(id).unwrap()))
    }

    pub fn into_u64(self) -> u64 {
        self.0.into_u64()
    }

    pub(super) fn from_u64(value: u64) -> Option<Self> {
        SerializableNonZeroU64::from_u64(value).map(Self)
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::next()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Serialize))]
pub struct CollectedSpan {
    pub(crate) id: SpanId,
    pub(crate) parent_id: Option<SpanId>,
    pub(crate) name: Cow<'static, str>,
    #[cfg_attr(test, serde(skip_serializing))]
    pub(crate) start_time: SystemTime,
    #[cfg_attr(test, serde(skip_serializing))]
    pub(crate) duration: Duration,
    pub(crate) attributes: HashMap<&'static str, serde_json::Value>,
    pub(crate) kind: SpanKind,
    pub(crate) links: Vec<SpanId>,
}

pub(crate) struct SpanBuilder {
    request_id: Option<RequestId>,
    id: SpanId,
    name: Cow<'static, str>,
    // we store both the wall clock time and a monotonically increasing instant to
    // be resilient against clock changes between the start and end of the span
    start_time: SystemTime,
    start_instant: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
    kind: Option<SpanKind>,
    links: Vec<SpanId>,
}

impl SpanBuilder {
    pub fn new(name: &'static str, id: impl Into<SpanId>, attrs_size_hint: usize) -> Self {
        Self {
            request_id: None,
            id: id.into(),
            name: name.into(),
            start_time: SystemTime::now(),
            start_instant: Instant::now(),
            attributes: HashMap::with_capacity(attrs_size_hint),
            kind: None,
            links: Vec::new(),
        }
    }

    pub fn request_id(&self) -> Option<RequestId> {
        self.request_id
    }

    pub fn set_request_id(&mut self, request_id: RequestId) {
        self.request_id = Some(request_id);
    }

    pub fn set_name(&mut self, name: Cow<'static, str>) {
        self.name = name;
    }

    pub fn set_kind(&mut self, kind: SpanKind) {
        self.kind = Some(kind);
    }

    pub fn insert_attribute(&mut self, key: &'static str, value: serde_json::Value) {
        self.attributes.insert(key, value);
    }

    pub fn add_link(&mut self, link: SpanId) {
        self.links.push(link);
    }

    pub fn end(self, parent_id: Option<impl Into<SpanId>>) -> CollectedSpan {
        CollectedSpan {
            id: self.id,
            parent_id: parent_id.map(Into::into),
            name: self.name,
            start_time: self.start_time,
            duration: self.start_instant.elapsed(),
            attributes: self.attributes,
            kind: self.kind.unwrap_or(SpanKind::Internal),
            links: self.links,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Serialize))]
pub struct CollectedEvent {
    pub(crate) span_id: SpanId,
    pub(crate) name: &'static str,
    pub(crate) level: LogLevel,
    #[cfg_attr(test, serde(skip_serializing))]
    pub(crate) timestamp: SystemTime,
    pub(crate) attributes: HashMap<&'static str, serde_json::Value>,
}

pub(crate) struct EventBuilder {
    span_id: SpanId,
    name: &'static str,
    level: LogLevel,
    timestamp: SystemTime,
    attributes: HashMap<&'static str, serde_json::Value>,
}

impl EventBuilder {
    pub fn new(span_id: SpanId, name: &'static str, level: LogLevel, attrs_size_hint: usize) -> Self {
        Self {
            span_id,
            name,
            level,
            timestamp: SystemTime::now(),
            attributes: HashMap::with_capacity(attrs_size_hint),
        }
    }

    pub fn insert_attribute(&mut self, key: &'static str, value: serde_json::Value) {
        self.attributes.insert(key, value);
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    pub fn build(self) -> CollectedEvent {
        CollectedEvent {
            span_id: self.span_id,
            name: self.name,
            level: self.level,
            timestamp: self.timestamp,
            attributes: self.attributes,
        }
    }
}

pub trait Collector {
    fn add_span(&self, trace: RequestId, span: CollectedSpan);
    fn add_event(&self, trace: RequestId, event: CollectedEvent);
}
