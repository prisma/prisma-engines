use std::{borrow::Cow, time::Duration};

use ahash::{HashMap, HashMapExt};
use crosstarget_utils::time::{ElapsedTimeCounter, SystemTime};
#[cfg(test)]
use serde::Serialize;

use crate::id::{RequestId, SpanId};
use crate::models::{LogLevel, SpanKind};

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
    // we store both the wall clock time and a monotonically increasing counter to
    // be resilient against clock changes between the start and end of the span
    start_time: SystemTime,
    elapsed: ElapsedTimeCounter,
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
            elapsed: ElapsedTimeCounter::start(),
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
            duration: self.elapsed.elapsed_time(),
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
    pub(crate) target: &'static str,
    pub(crate) level: LogLevel,
    #[cfg_attr(test, serde(skip_serializing))]
    pub(crate) timestamp: SystemTime,
    pub(crate) attributes: HashMap<&'static str, serde_json::Value>,
}

pub(crate) struct EventBuilder {
    span_id: SpanId,
    target: &'static str,
    level: LogLevel,
    timestamp: SystemTime,
    attributes: HashMap<&'static str, serde_json::Value>,
}

impl EventBuilder {
    pub fn new(span_id: SpanId, target: &'static str, level: LogLevel, attrs_size_hint: usize) -> Self {
        Self {
            span_id,
            target,
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
            target: self.target,
            level: self.level,
            timestamp: self.timestamp,
            attributes: self.attributes,
        }
    }
}

/// A trait for collecting spans and events from [`CapturingLayer`][crate::layer::CapturingLayer].
pub trait Collector {
    type AttributeFilter: AllowAttribute;
    fn add_span(&self, trace: RequestId, span: CollectedSpan);
    fn add_event(&self, trace: RequestId, event: CollectedEvent);
}

/// Filters span and event attributes based on the attribute name.
///
/// This trait is used by the [`CapturingLayer`][crate::layer::CapturingLayer]
/// as an associated type on the [`Collector`]. This way the collector can
/// define which attributes should be kept and which should be filtered out but
/// it doesn't have to implement the filtering in `add_span` and `add_event`
/// methods and remove any attributes from [`CollectedSpan`] and
/// [`CollectedEvent`]. Instead, those attributes are filtered out before they
/// are even passed to the collector and are never stored anywhere except the
/// original [`tracing::Span`].
///
/// The magic attributes used by the `CapturingLayer` itself (i.e. `request_id`
/// and `otel.*`) are not collected as attributes and thus don't need to be
/// explicitly filtered out.
pub trait AllowAttribute {
    fn allow_on_span(name: &'static str) -> bool;
    fn allow_on_event(name: &'static str) -> bool;
}

pub struct DefaultAttributeFilter;

impl AllowAttribute for DefaultAttributeFilter {
    fn allow_on_span(_name: &'static str) -> bool {
        true
    }

    fn allow_on_event(_name: &'static str) -> bool {
        true
    }
}
