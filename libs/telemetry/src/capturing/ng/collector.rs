use std::{
    borrow::Cow,
    collections::HashMap,
    num::NonZeroU64,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use derive_more::Display;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use tracing::Level;

use crate::models::{LogLevel, SpanKind, TraceSpan};

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(fmt = "{}", _0)]
struct SerializableNonZeroU64(NonZeroU64);

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

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(fmt = "{}", _0)]
struct SerializableU64(u64);

impl Serialize for SerializableU64 {
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

impl<'de> Deserialize<'de> for SerializableU64 {
    fn deserialize<D>(deserializer: D) -> Result<SerializableU64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let value = value.parse().map_err(serde::de::Error::custom)?;
        Ok(SerializableU64(value))
    }
}

/// A unique identifier for a span. It maps directly to [`tracing::span::Id`] assigned by
/// [`tracing_subscriber::registry::Registry`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
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
pub struct RequestId(SerializableU64);

impl RequestId {
    pub fn next() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(SerializableU64(NEXT_ID.fetch_add(1, Ordering::Relaxed)))
    }

    pub fn into_u64(self) -> u64 {
        self.0 .0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::next()
    }
}

impl From<u64> for RequestId {
    fn from(id: u64) -> Self {
        Self(SerializableU64(id))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Serialize))]
pub struct CollectedSpan {
    id: SpanId,
    parent_id: Option<SpanId>,
    name: Cow<'static, str>,
    #[cfg_attr(test, serde(skip_serializing))]
    start_time: Instant,
    #[cfg_attr(test, serde(skip_serializing))]
    end_time: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
    kind: SpanKind,
    links: Vec<SpanId>,
}

pub(crate) struct SpanBuilder {
    request_id: Option<RequestId>,
    id: SpanId,
    name: Cow<'static, str>,
    start_time: Instant,
    end_time: Option<Instant>,
    attributes: HashMap<&'static str, serde_json::Value>,
    kind: Option<SpanKind>,
    links: Vec<SpanId>,
}

impl SpanBuilder {
    pub fn new(name: &'static str, id: impl Into<SpanId>, start_time: Instant, attrs_size_hint: usize) -> Self {
        Self {
            request_id: None,
            id: id.into(),
            name: name.into(),
            start_time,
            end_time: None,
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

    pub fn end(self, parent_id: Option<impl Into<SpanId>>, end_time: Instant) -> CollectedSpan {
        CollectedSpan {
            id: self.id,
            parent_id: parent_id.map(Into::into),
            name: self.name,
            start_time: self.start_time,
            end_time,
            attributes: self.attributes,
            kind: self.kind.unwrap_or(SpanKind::Internal),
            links: self.links,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Serialize))]
pub struct CollectedEvent {
    span_id: SpanId,
    name: &'static str,
    level: LogLevel,
    #[cfg_attr(test, serde(skip_serializing))]
    timestamp: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
}

pub(crate) struct EventBuilder {
    span_id: SpanId,
    name: &'static str,
    level: LogLevel,
    timestamp: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
}

impl EventBuilder {
    pub fn new(
        span_id: SpanId,
        name: &'static str,
        level: LogLevel,
        timestamp: Instant,
        attrs_size_hint: usize,
    ) -> Self {
        Self {
            span_id,
            name,
            level,
            timestamp,
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

#[derive(Clone)]
pub struct Exporter(Arc<ExporterInner>);

struct ExporterInner {
    tasks: HashMap<SpanId, ()>,
}

impl Exporter {
    pub fn new() -> Self {
        Self(Arc::new(ExporterInner { tasks: HashMap::new() }))
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for Exporter {
    fn add_span(&self, _trace: RequestId, _span: CollectedSpan) {
        todo!()
    }

    fn add_event(&self, _trace: RequestId, _event: CollectedEvent) {
        todo!()
    }
}
